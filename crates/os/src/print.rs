//! The platform's native print dialog for a PDF (B5b).
//!
//! RustFig makes the PDF of the figure (fig2dev's output on the page the Print
//! panel shows); this shows the operating system's own print dialog for it, so
//! printer, copies, page ranges and options are the system's, not RustFig's.
//!
//! - macOS: PDFKit's `PDFDocument` print operation (`NSPrintOperation` with the
//!   standard print panel), on the main thread (the egui app's own).
//! - Linux/BSD: the xdg-desktop-portal `Print` interface: `PreparePrint` shows
//!   the dialog, `Print` sends the PDF (a file descriptor); no unsafe.
//! - Windows: the common print dialog (`PrintDlgEx`), then the PDF goes to the
//!   chosen printer through the shell's `printto` verb (the default PDF
//!   viewer prints it). Copies and page ranges of the dialog are not passed on.
//! - Web: the PDF opens in a new tab (blob URL), where the browser's viewer
//!   prints it.

/// How the dialog ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintOutcome {
    /// The job was handed to the system's print system.
    Printed,
    /// The user cancelled the dialog.
    Cancelled,
}

/// Show the platform's print dialog for `pdf`; `job_name` is the job's title.
/// Blocks until the dialog is closed (call it from the main thread: macOS
/// requires it). Errors carry a text for the message line.
pub fn print_pdf(pdf: &[u8], job_name: &str) -> Result<PrintOutcome, String> {
    if !pdf.starts_with(b"%PDF") {
        return Err("the print job is not a PDF".into());
    }
    platform_print(pdf, job_name)
}

/// A file name made of the job's name: ASCII letters, digits, `-`, `_`, `.`.
#[cfg_attr(any(target_os = "macos", target_arch = "wasm32"), allow(dead_code))]
fn safe_name(job_name: &str) -> String {
    let n: String = job_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    let n = n.trim_start_matches('.');
    if n.is_empty() {
        "figure".into()
    } else {
        n.into()
    }
}

/// A fresh temporary file for the PDF; the caller removes it.
#[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
fn write_temp(pdf: &[u8], job_name: &str) -> Result<std::path::PathBuf, String> {
    let dir = std::env::temp_dir().join("rustfig-print");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let path = dir.join(format!("{stamp}-{}-{}.pdf", std::process::id(), safe_name(job_name)));
    std::fs::write(&path, pdf).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(target_os = "macos")]
fn platform_print(pdf: &[u8], job_name: &str) -> Result<PrintOutcome, String> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSPrintInfo;
    use objc2_foundation::{NSData, NSString};
    use objc2_pdf_kit::PDFPrintScalingMode;

    let Some(mtm) = MainThreadMarker::new() else {
        return Err("The print dialog must open on the main thread".into());
    };
    let data = NSData::with_bytes(pdf);
    let doc = pdf_document(&data)?;
    // The shared print info is the app's page setup (paper and margins of the user's default printer).
    let info = NSPrintInfo::sharedPrintInfo();
    let op = pdf_print_operation(&doc, &info, PDFPrintScalingMode::PageScaleToFit, mtm)
        .ok_or_else(|| "The system could not prepare the print job".to_string())?;
    op.setJobTitle(Some(&NSString::from_str(job_name)));
    op.setShowsPrintPanel(true);
    op.setShowsProgressPanel(true);
    // YES: printed (or saved as PDF from the panel); NO: cancelled or failed.
    if op.runOperation() {
        Ok(PrintOutcome::Printed)
    } else {
        Ok(PrintOutcome::Cancelled)
    }
}

/// The PDFKit document of `data`.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn pdf_document(data: &objc2_foundation::NSData) -> Result<objc2::rc::Retained<objc2_pdf_kit::PDFDocument>, String> {
    use objc2::AnyThread;
    // SAFETY: `data` is a valid NSData for the call; initWithData: returns nil for data that is not a PDF.
    unsafe { objc2_pdf_kit::PDFDocument::initWithData(objc2_pdf_kit::PDFDocument::alloc(), data) }
        .ok_or_else(|| "The PDF cannot be read by the system".to_string())
}

/// PDFKit's print operation for `doc`.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn pdf_print_operation(
    doc: &objc2_pdf_kit::PDFDocument,
    info: &objc2_app_kit::NSPrintInfo,
    scale: objc2_pdf_kit::PDFPrintScalingMode,
    mtm: objc2::MainThreadMarker,
) -> Option<objc2::rc::Retained<objc2_app_kit::NSPrintOperation>> {
    // SAFETY: `doc` and `info` are valid objects, called on the main thread (`mtm`); the method
    // returns nil when it cannot make an operation.
    unsafe { doc.printOperationForPrintInfo_scalingMode_autoRotate(Some(info), scale, true, mtm) }
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn platform_print(pdf: &[u8], job_name: &str) -> Result<PrintOutcome, String> {
    use ashpd::desktop::print::{PageSetup, PrintProxy, Settings};
    use ashpd::desktop::ResponseError;

    let path = write_temp(pdf, job_name)?;
    let result = (|| {
        let file = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let title = if job_name.is_empty() { "Print" } else { job_name };
        pollster::block_on(async {
            let proxy = PrintProxy::new().await?;
            let prepared = proxy
                .prepare_print(None, title, Settings::default(), PageSetup::default(), None, true)
                .await?
                .response()?;
            proxy
                .print(None, title, &file, Some(prepared.token), true)
                .await?
                .response()?;
            Ok::<_, ashpd::Error>(())
        })
        .map(|()| PrintOutcome::Printed)
        .or_else(|e| match e {
            ashpd::Error::Response(ResponseError::Cancelled) => Ok(PrintOutcome::Cancelled),
            ashpd::Error::PortalNotFound(_) => Err(
                "Printing needs xdg-desktop-portal with a Print backend (GTK or KDE portal), which was not found"
                    .to_string(),
            ),
            e => Err(format!("The print portal failed: {e}")),
        })
    })();
    // The portal has read the file by now (it holds its own descriptor); a leftover is harmless.
    let _ = std::fs::remove_file(&path);
    result
}

#[cfg(target_os = "windows")]
fn platform_print(pdf: &[u8], job_name: &str) -> Result<PrintOutcome, String> {
    let Some(printer) = win::choose_printer()? else {
        return Ok(PrintOutcome::Cancelled);
    };
    // The viewer reads the file after this returns, so it stays in the temp folder (rustfig-print).
    let path = write_temp(pdf, job_name)?;
    win::print_to(&path, &printer)?;
    Ok(PrintOutcome::Printed)
}

#[cfg(target_os = "windows")]
mod win {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows_sys::Win32::UI::Controls::Dialogs::{
        PrintDlgExW, DEVNAMES, PD_NOCURRENTPAGE, PD_NOSELECTION, PD_RESULT_CANCEL, PD_USEDEVMODECOPIESANDCOLLATE,
        PRINTDLGEXW, PRINTPAGERANGE, START_PAGE_GENERAL,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetDesktopWindow, SW_HIDE};

    fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Show the common print dialog; the chosen printer's name, `None` when cancelled.
    #[allow(unsafe_code)]
    pub fn choose_printer() -> Result<Option<String>, String> {
        // SAFETY: CoInitializeEx takes a null reserved pointer; a failure (COM already set up in
        // another mode by the windowing library) leaves COM usable for the dialog.
        unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
        let mut ranges = [PRINTPAGERANGE {
            nFromPage: 1,
            nToPage: 1,
        }];
        // SAFETY: PRINTDLGEXW is plain data; all-zero is its documented "empty" state.
        let mut dlg: PRINTDLGEXW = unsafe { std::mem::zeroed() };
        dlg.lStructSize = std::mem::size_of::<PRINTDLGEXW>() as u32;
        // SAFETY: both calls have no preconditions; the dialog needs a valid owner window.
        dlg.hwndOwner = unsafe {
            let w = GetActiveWindow();
            if w.is_null() {
                GetDesktopWindow()
            } else {
                w
            }
        };
        dlg.Flags = PD_NOCURRENTPAGE | PD_NOSELECTION | PD_USEDEVMODECOPIESANDCOLLATE;
        dlg.nMaxPageRanges = 1;
        dlg.lpPageRanges = ranges.as_mut_ptr();
        dlg.nMinPage = 1;
        dlg.nMaxPage = 9999;
        dlg.nCopies = 1;
        dlg.nStartPage = START_PAGE_GENERAL;
        // SAFETY: `dlg` is initialised as the API requires and `ranges` outlives the call.
        let hr = unsafe { PrintDlgExW(&mut dlg) };
        if hr < 0 {
            return Err(format!("The print dialog failed (HRESULT {hr:#x})"));
        }
        let name = device_name(dlg.hDevNames);
        // SAFETY: the handles came from the dialog and are freed once; null is skipped.
        unsafe {
            if !dlg.hDevNames.is_null() {
                GlobalFree(dlg.hDevNames);
            }
            if !dlg.hDevMode.is_null() {
                GlobalFree(dlg.hDevMode);
            }
        }
        if dlg.dwResultAction == PD_RESULT_CANCEL {
            return Ok(None);
        }
        name.map(Some)
            .ok_or_else(|| "The print dialog gave no printer".to_string())
    }

    /// The device (printer) name in a DEVNAMES block.
    #[allow(unsafe_code)]
    fn device_name(h: windows_sys::Win32::Foundation::HGLOBAL) -> Option<String> {
        if h.is_null() {
            return None;
        }
        // SAFETY: `h` is the DEVNAMES block the dialog returned; the offsets are in wide characters
        // from its start and the strings are NUL-terminated, so reads stay inside the block.
        unsafe {
            let p = GlobalLock(h) as *const u16;
            if p.is_null() {
                return None;
            }
            let names = &*(p as *const DEVNAMES);
            let mut s = p.add(usize::from(names.wDeviceOffset));
            let mut v = Vec::new();
            while *s != 0 && v.len() < 512 {
                v.push(*s);
                s = s.add(1);
            }
            GlobalUnlock(h);
            Some(String::from_utf16_lossy(&v)).filter(|n| !n.is_empty())
        }
    }

    /// Print `path` on `printer` through the shell: `printto`, else `print`.
    #[allow(unsafe_code)]
    pub fn print_to(path: &std::path::Path, printer: &str) -> Result<(), String> {
        let file = wide(path.as_os_str());
        let args = wide(std::ffi::OsStr::new(&format!("\"{printer}\"")));
        for (verb, with_args) in [("printto", true), ("print", false)] {
            let v = wide(std::ffi::OsStr::new(verb));
            let a = if with_args { args.as_ptr() } else { std::ptr::null() };
            // SAFETY: all strings are NUL-terminated wide strings that outlive the call; null is
            // allowed for the window, parameters and directory.
            let r = unsafe {
                ShellExecuteW(
                    std::ptr::null_mut(),
                    v.as_ptr(),
                    file.as_ptr(),
                    a,
                    std::ptr::null(),
                    SW_HIDE,
                )
            };
            // ShellExecute returns a value above 32 on success.
            if r as usize > 32 {
                return Ok(());
            }
        }
        Err("No program is registered to print PDF files; install a PDF viewer".into())
    }
}

#[cfg(target_arch = "wasm32")]
fn platform_print(pdf: &[u8], _job_name: &str) -> Result<PrintOutcome, String> {
    use js_sys::{Array, Uint8Array};
    let window = web_sys::window().ok_or("no browser window")?;
    let bytes = Uint8Array::from(pdf);
    let parts = Array::of1(&bytes);
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/pdf");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts)
        .map_err(|_| "the browser could not make the PDF".to_string())?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "the browser could not make the PDF".to_string())?;
    if print_in_frame(&window, &url).is_some() {
        return Ok(PrintOutcome::Printed);
    }
    match window.open_with_url_and_target(&url, "_blank") {
        Ok(Some(_)) => Ok(PrintOutcome::Printed),
        _ => Err("The browser blocked the new tab; allow pop-ups for this page".into()),
    }
}

/// Load the PDF in an invisible iframe and open the browser's print dialog
/// for it once it has loaded. `None` when the page can't hold the frame (the
/// caller opens a tab instead). If the browser refuses `print()` on the
/// frame (some PDF viewers), the frame's own load handler opens the tab.
#[cfg(target_arch = "wasm32")]
fn print_in_frame(window: &web_sys::Window, url: &str) -> Option<()> {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    const ID: &str = "rustfig-print-frame";
    let document = window.document()?;
    if let Some(old) = document.get_element_by_id(ID) {
        old.remove();
    }
    let frame: web_sys::HtmlIFrameElement = document.create_element("iframe").ok()?.dyn_into().ok()?;
    frame.set_id(ID);
    frame
        .set_attribute(
            "style",
            "position:fixed;right:0;bottom:0;width:0;height:0;border:0;visibility:hidden",
        )
        .ok()?;
    let target = window.clone();
    let target_url = url.to_string();
    let frame_for_load = frame.clone();
    let on_load = Closure::once_into_js(move || {
        let printed = frame_for_load
            .content_window()
            .is_some_and(|w| w.focus().is_ok() && w.print().is_ok());
        if !printed {
            // Ignored on purpose: nothing else can be done if this is blocked too.
            let _ = target.open_with_url_and_target(&target_url, "_blank");
        }
    });
    frame.set_onload(Some(on_load.unchecked_ref()));
    frame.set_src(url);
    document.body()?.append_child(&frame).ok()?;
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_what_is_not_a_pdf() {
        assert!(print_pdf(b"hello", "x").is_err());
    }

    #[test]
    fn names_are_safe() {
        assert_eq!(safe_name("my fig/1"), "my_fig_1");
        assert_eq!(safe_name("..."), "figure");
        assert_eq!(safe_name(""), "figure");
    }
}
