//! Files the system asks the running program to open (P3): a double-click in Finder, `open -a
//! RustFig x.fig`, a drop on the Dock icon.
//!
//! - macOS: an Apple Event handler for `aevt`/`odoc` (winit's app delegate has no
//!   `application:openURLs:`, so the event would be dropped). AppKit registers its own handler
//!   in `finishLaunching`; ours replaces it when `NSApplicationWillFinishLaunchingNotification`
//!   arrives, before the launch documents are delivered. The paths queue up; the UI takes them
//!   every frame with [`take_opened_files`].
//! - Elsewhere: nothing to do (the files come in `argv`), both functions are no-ops.
//!
//! The OS calls are `unsafe`, allowed per function with a SAFETY comment; the API is safe.

use std::path::PathBuf;
use std::sync::Mutex;

static QUEUE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Cap on queued paths (a hostile sender must not grow memory without bound).
const MAX_QUEUED: usize = 1000;

/// Add a path to the queue (a poisoned lock is recovered: the queue is plain data).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn push(path: PathBuf) {
    let mut q = QUEUE.lock().unwrap_or_else(|e| e.into_inner());
    if q.len() < MAX_QUEUED {
        q.push(path);
    }
}

/// Files the system asked us to open since the last call, in order. Cheap when there are none.
pub fn take_opened_files() -> Vec<PathBuf> {
    match QUEUE.lock() {
        Ok(mut q) => std::mem::take(&mut *q),
        Err(e) => std::mem::take(&mut *e.into_inner()),
    }
}

/// Start listening for the system's open-document requests. Call once, on the main thread, as
/// early as possible (before the event loop is created). No-op except on macOS.
pub fn install_open_handler() {
    #[cfg(target_os = "macos")]
    mac::install();
}

#[cfg(target_os = "macos")]
mod mac {
    // `define_class!` expands to `unsafe` code; the calls below carry their SAFETY comments.
    #![allow(unsafe_code)]

    use super::push;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, NSObject};
    use objc2::{define_class, msg_send, sel, AnyThread};
    use objc2_foundation::{NSInteger, NSString, NSURL};
    use std::path::PathBuf;
    use std::sync::Once;

    // FourCharCodes: 'aevt', 'odoc', '----' (direct object).
    const KEVENT_CLASS: u32 = 0x6165_7674;
    const KEVENT_ID: u32 = 0x6F64_6F63;
    const KEY_DIRECT: u32 = 0x2D2D_2D2D;

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "RustFigOpenHandler"]
        struct Handler;

        impl Handler {
            #[unsafe(method(handleOpen:withReply:))]
            fn handle_open(&self, event: &AnyObject, _reply: &AnyObject) {
                read_event(event);
            }

            #[unsafe(method(willFinishLaunching:))]
            fn will_finish(&self, _note: &AnyObject) {
                register_handler(self);
            }
        }
    );

    /// The file paths in an `odoc` event's direct object (a list of file URLs).
    fn read_event(event: &AnyObject) {
        // SAFETY: `event` is the NSAppleEventDescriptor AppKit passes to the handler; the
        // selectors exist on it and the argument/return types match their declarations.
        unsafe {
            let list: Option<Retained<AnyObject>> = msg_send![event, paramDescriptorForKeyword: KEY_DIRECT];
            let Some(list) = list else { return };
            let n: NSInteger = msg_send![&*list, numberOfItems];
            for i in 1..=n.min(1000) {
                let item: Option<Retained<AnyObject>> = msg_send![&*list, descriptorAtIndex: i];
                let Some(item) = item else { continue };
                let text: Option<Retained<NSString>> = msg_send![&*item, stringValue];
                let Some(text) = text else { continue };
                // The descriptor holds a `file://` URL.
                if let Some(path) = NSURL::URLWithString(&text).and_then(|u| u.path()) {
                    push(PathBuf::from(path.to_string()));
                }
            }
        }
    }

    fn register_handler(handler: &Handler) {
        let Some(cls) = AnyClass::get(c"NSAppleEventManager") else {
            return;
        };
        // SAFETY: `handler` implements `handleOpen:withReply:` (the selector we pass) and is
        // leaked in `install`, so it outlives the manager's unretained reference.
        unsafe {
            let mgr: Option<Retained<AnyObject>> = msg_send![cls, sharedAppleEventManager];
            if let Some(mgr) = mgr {
                let _: () = msg_send![
                    &*mgr,
                    setEventHandler: handler,
                    andSelector: sel!(handleOpen:withReply:),
                    forEventClass: KEVENT_CLASS,
                    andEventID: KEVENT_ID
                ];
            }
        }
    }

    pub fn install() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let this = Handler::alloc();
            // SAFETY: plain `init` of an NSObject subclass without ivars.
            let handler: Retained<Handler> = unsafe { msg_send![this, init] };
            let Some(center_cls) = AnyClass::get(c"NSNotificationCenter") else {
                return;
            };
            let name = NSString::from_str("NSApplicationWillFinishLaunchingNotification");
            // SAFETY: `willFinishLaunching:` takes one object argument, as the notification
            // centre passes; the handler is leaked below so the unretained observer stays valid.
            unsafe {
                let center: Option<Retained<AnyObject>> = msg_send![center_cls, defaultCenter];
                if let Some(center) = center {
                    let none: Option<&AnyObject> = None;
                    let _: () = msg_send![
                        &*center,
                        addObserver: &*handler,
                        selector: sel!(willFinishLaunching:),
                        name: &*name,
                        object: none
                    ];
                }
            }
            // Lives for the whole run: AppKit and the notification centre keep unretained pointers.
            std::mem::forget(handler);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_takes_in_order_and_empties() {
        assert!(take_opened_files().is_empty());
        push(PathBuf::from("/a.fig"));
        push(PathBuf::from("/b.fig"));
        let got = take_opened_files();
        assert_eq!(got, vec![PathBuf::from("/a.fig"), PathBuf::from("/b.fig")]);
        assert!(take_opened_files().is_empty());
    }
}
