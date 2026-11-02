//! macOS Quick Look extensions (P5): the entry point of the thumbnail and preview app
//! extensions (`.appex`) that `cargo xtask package` embeds in `RustFig.app/Contents/PlugIns`.
//!
//! An app extension is an executable whose `main` hands over to Foundation's
//! `NSExtensionMain`; that creates the class named by `NSExtensionPrincipalClass` in the
//! extension's `Info.plist`. Both classes are declared here at run time (no Xcode project, no
//! Swift): `RustFigThumbnailProvider` (a `QLThumbnailProvider`, Finder icons and Spotlight) and
//! `RustFigPreviewProvider` (a `QLPreviewProvider`, data-based preview: space bar, Finder
//! columns). Both ask the `render` function the caller gives for PNG bytes of the figure, so this
//! crate knows nothing of the engine.
//!
//! The OS calls are `unsafe`, allowed per function with a SAFETY comment; the API is safe.
//! Elsewhere [`run_extension`] only reports that there is nothing to run.

use std::path::Path;

/// Renders the figure at `path` into PNG bytes whose larger side is at most the given pixels.
pub type RenderFn = fn(&Path, u32) -> Result<Vec<u8>, String>;

/// Run the extension: never returns on macOS when started by the system (NSExtensionMain
/// runs the extension's event loop); returns an exit code if it could not start.
pub fn run_extension(render: RenderFn) -> i32 {
    #[cfg(target_os = "macos")]
    {
        mac::run(render)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = render;
        eprintln!("rustfig-quicklook: Quick Look extensions exist on macOS only");
        1
    }
}

/// Width and height from a PNG's IHDR chunk.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..8)? != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let h = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((w, h))
}

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unsafe_code)]

    use super::{png_size, RenderFn};
    use block2::{Block, RcBlock};
    use objc2::encode::{Encode, Encoding, RefEncode};
    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, Sel};
    use objc2::{msg_send, sel};
    use objc2_foundation::{NSInteger, NSString};
    use std::ffi::{c_char, c_int};
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    #[link(name = "Foundation", kind = "framework")]
    unsafe extern "C" {
        fn NSExtensionMain(argc: c_int, argv: *const *const c_char) -> c_int;
    }

    // The provider classes come from these frameworks; one block each, as
    // clippy rejects several `link` attributes on one block.
    #[link(name = "QuickLook", kind = "framework")]
    unsafe extern "C" {}
    #[link(name = "QuickLookThumbnailing", kind = "framework")]
    unsafe extern "C" {}
    #[link(name = "UniformTypeIdentifiers", kind = "framework")]
    unsafe extern "C" {}

    static RENDER: OnceLock<RenderFn> = OnceLock::new();

    /// Largest preview edge in pixels.
    const PREVIEW_PX: u32 = 1600;
    const MIN_PX: u32 = 16;
    const MAX_THUMB_PX: u32 = 2048;

    /// `CGSize` (two doubles), the type of `maximumSize` and of the preview's `contentSize`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }
    // SAFETY: the encoding is that of CoreGraphics' CGSize struct, which has this layout.
    unsafe impl Encode for CGSize {
        const ENCODING: Encoding = Encoding::Struct("CGSize", &[Encoding::Double, Encoding::Double]);
    }
    // SAFETY: as above, for a pointer to it.
    unsafe impl RefEncode for CGSize {
        const ENCODING_REF: Encoding = Encoding::Pointer(&<Self as Encode>::ENCODING);
    }

    type ThumbDone = Block<dyn Fn(*mut AnyObject, *mut AnyObject)>;

    pub fn run(render: RenderFn) -> i32 {
        let _ = RENDER.set(render);
        if !declare() {
            eprintln!("rustfig-quicklook: Quick Look classes not available");
            return 1;
        }
        // NSExtensionMain reads the plist, creates the principal class and runs forever;
        // it takes the arguments the system gave `main`.
        let args: Vec<std::ffi::CString> = std::env::args_os()
            .filter_map(|a| std::ffi::CString::new(a.as_encoded_bytes()).ok())
            .collect();
        let mut argv: Vec<*const c_char> = args.iter().map(|a| a.as_ptr()).collect();
        argv.push(std::ptr::null());
        // SAFETY: argv is a NULL-terminated array of NUL-terminated strings that outlive the
        // call, argc is its length; NSExtensionMain is Foundation's documented extension entry.
        unsafe { NSExtensionMain(args.len() as c_int, argv.as_ptr()) }
    }

    /// Register the two provider classes. False when a superclass is missing.
    fn declare() -> bool {
        let thumb =
            AnyClass::get(c"QLThumbnailProvider").and_then(|sup| ClassBuilder::new(c"RustFigThumbnailProvider", sup));
        let preview =
            AnyClass::get(c"QLPreviewProvider").and_then(|sup| ClassBuilder::new(c"RustFigPreviewProvider", sup));
        let (Some(mut thumb), Some(mut preview)) = (thumb, preview) else {
            return false;
        };
        // SAFETY: the selectors are the protocol methods of the superclasses; the function
        // signatures (self, _cmd, request, completion block) match their declarations.
        unsafe {
            thumb.add_method(
                sel!(provideThumbnailForFileRequest:completionHandler:),
                provide_thumbnail as extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject),
            );
            preview.add_method(
                sel!(providePreviewForFileRequest:completionHandler:),
                provide_preview as extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject),
            );
        }
        thumb.register();
        preview.register();
        true
    }

    fn error(message: &str) -> Retained<AnyObject> {
        // SAFETY: NSError class methods with the documented argument types; a nil userInfo is allowed.
        unsafe {
            let domain = NSString::from_str("RustFigQuickLook");
            let key = NSString::from_str("NSLocalizedDescription");
            let text = NSString::from_str(message);
            let info: Option<Retained<AnyObject>> = msg_send![
                AnyClass::get(c"NSDictionary").map_or(std::ptr::null(), |c| c as *const AnyClass),
                dictionaryWithObject: &*text,
                forKey: &*key
            ];
            let err: Retained<AnyObject> = msg_send![
                AnyClass::get(c"NSError").map_or(std::ptr::null(), |c| c as *const AnyClass),
                errorWithDomain: &*domain,
                code: 1 as NSInteger,
                userInfo: info.as_deref()
            ];
            err
        }
    }

    /// The request's file path (`fileURL.path`).
    fn request_path(request: &AnyObject) -> Option<PathBuf> {
        // SAFETY: QLFileThumbnailRequest and QLFilePreviewRequest both have `fileURL` (NSURL),
        // which answers `path` (NSString).
        unsafe {
            let url: Option<Retained<AnyObject>> = msg_send![request, fileURL];
            let path: Option<Retained<NSString>> = msg_send![&*url?, path];
            Some(PathBuf::from(path?.to_string()))
        }
    }

    fn render(path: &Path, px: u32) -> Result<Vec<u8>, String> {
        let f = RENDER.get().ok_or("no renderer")?;
        f(path, px)
    }

    extern "C-unwind" fn provide_thumbnail(
        _this: *mut AnyObject,
        _cmd: Sel,
        request: *mut AnyObject,
        done: *mut AnyObject,
    ) {
        // SAFETY: the system passes a request object and a completion block, both valid for the
        // call (and, for the block, until it has been called); a null pointer is refused.
        let (Some(request), Some(done)) = (unsafe { request.as_ref() }, unsafe {
            (done as *const ThumbDone).as_ref()
        }) else {
            return;
        };
        let result = thumbnail_reply(request);
        match result {
            Ok(reply) => done.call((Retained::as_ptr(&reply).cast_mut(), std::ptr::null_mut())),
            Err(msg) => {
                let err = error(&msg);
                done.call((std::ptr::null_mut(), Retained::as_ptr(&err).cast_mut()));
            }
        }
    }

    fn thumbnail_reply(request: &AnyObject) -> Result<Retained<AnyObject>, String> {
        let path = request_path(request).ok_or("no file")?;
        // SAFETY: `maximumSize` is a CGSize (points), `scale` a CGFloat.
        let (max, scale): (CGSize, f64) = unsafe { (msg_send![request, maximumSize], msg_send![request, scale]) };
        let side = max.width.max(max.height) * if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
        let px = if side.is_finite() { side as u32 } else { 256 }.clamp(MIN_PX, MAX_THUMB_PX);
        let png = render(&path, px)?;
        let tmp = std::env::temp_dir().join(format!("rustfig-ql-{}-{}.png", std::process::id(), px));
        std::fs::write(&tmp, &png).map_err(|e| e.to_string())?;
        let text = NSString::from_str(&tmp.to_string_lossy());
        // SAFETY: `fileURLWithPath:` and `replyWithImageFileURL:` take and return objects as declared;
        // the reply copies nothing until the system reads the file, which stays in the sandbox's
        // temporary folder (the system removes it with the container's temp files).
        unsafe {
            let url: Option<Retained<AnyObject>> = msg_send![
                AnyClass::get(c"NSURL").map_or(std::ptr::null(), |c| c as *const AnyClass),
                fileURLWithPath: &*text
            ];
            let url = url.ok_or("no URL")?;
            let reply: Option<Retained<AnyObject>> = msg_send![
                AnyClass::get(c"QLThumbnailReply").map_or(std::ptr::null(), |c| c as *const AnyClass),
                replyWithImageFileURL: &*url
            ];
            reply.ok_or_else(|| "no reply".to_string())
        }
    }

    extern "C-unwind" fn provide_preview(
        _this: *mut AnyObject,
        _cmd: Sel,
        request: *mut AnyObject,
        done: *mut AnyObject,
    ) {
        // SAFETY: the system passes a request object and a completion block, both valid for the
        // call (and, for the block, until it has been called); a null pointer is refused.
        let (Some(request), Some(done)) = (unsafe { request.as_ref() }, unsafe {
            (done as *const ThumbDone).as_ref()
        }) else {
            return;
        };
        match preview_reply(request) {
            Ok(reply) => done.call((Retained::as_ptr(&reply).cast_mut(), std::ptr::null_mut())),
            Err(msg) => {
                let err = error(&msg);
                done.call((std::ptr::null_mut(), Retained::as_ptr(&err).cast_mut()));
            }
        }
    }

    fn preview_reply(request: &AnyObject) -> Result<Retained<AnyObject>, String> {
        let path = request_path(request).ok_or("no file")?;
        let png = render(&path, PREVIEW_PX)?;
        let (w, h) = png_size(&png).ok_or("the renderer did not make a PNG")?;
        let size = CGSize {
            width: f64::from(w),
            height: f64::from(h),
        };
        // SAFETY: NSData `dataWithBytes:length:` copies the bytes.
        let data: Retained<AnyObject> = unsafe {
            msg_send![
                AnyClass::get(c"NSData").map_or(std::ptr::null(), |c| c as *const AnyClass),
                dataWithBytes: png.as_ptr().cast::<std::ffi::c_void>(),
                length: png.len()
            ]
        };
        // The block returns the data autoreleased (+0), as Objective-C blocks do.
        let block = RcBlock::new(
            move |_reply: *mut AnyObject, _err: *mut *mut AnyObject| -> *mut AnyObject {
                Retained::as_ptr(&data).cast_mut()
            },
        );
        // SAFETY: UTType `typeWithIdentifier:` and QLPreviewReply
        // `initWithDataOfContentType:contentSize:dataCreationBlock:` as declared in
        // QuickLook/UniformTypeIdentifiers; the block is copied by the reply.
        unsafe {
            let ident = NSString::from_str("public.png");
            let ut: Option<Retained<AnyObject>> = msg_send![
                AnyClass::get(c"UTType").map_or(std::ptr::null(), |c| c as *const AnyClass),
                typeWithIdentifier: &*ident
            ];
            let ut = ut.ok_or("no UTType")?;
            let cls = AnyClass::get(c"QLPreviewReply").ok_or("no QLPreviewReply")?;
            let alloc: Allocated<AnyObject> = msg_send![cls, alloc];
            let reply: Option<Retained<AnyObject>> = msg_send![
                alloc,
                initWithDataOfContentType: &*ut,
                contentSize: size,
                dataCreationBlock: &*block
            ];
            reply.ok_or_else(|| "no reply".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_header_size() {
        let mut p = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        p.extend_from_slice(&7u32.to_be_bytes());
        p.extend_from_slice(&9u32.to_be_bytes());
        assert_eq!(png_size(&p), Some((7, 9)));
        assert_eq!(png_size(b"nope"), None);
    }
}
