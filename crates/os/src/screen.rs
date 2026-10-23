//! Pick a colour from the screen (E4b), also from other programs' windows.
//!
//! The OS calls below are `unsafe`, allowed per function; the API is safe and knows nothing of egui.
//!
//! - macOS: the system colour sampler (`NSColorSampler`), via [`start_sampler`].
//! - Linux/BSD: the xdg-desktop-portal `Screenshot.PickColor` request on a
//!   thread (no unsafe), via [`start_sampler`].
//! - Windows: no system sampler; the caller polls [`poll_pixel`] every frame
//!   (cursor position, `GetPixel`, Escape and left button state).
//! - Web: not available ([`available`] is false; the browser's EyeDropper needs JavaScript).

use std::sync::mpsc::Receiver;

/// How a pick ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The colour taken.
    Picked([u8; 3]),
    /// The user cancelled (Escape, closed the sampler).
    Cancelled,
    /// The system could not pick; the text goes to the message line.
    Failed(String),
}

/// What [`poll_pixel`] saw (Windows own picking).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PixelPoll {
    /// The colour under the pointer, when it could be read.
    pub colour: Option<[u8; 3]>,
    /// Escape is held down.
    pub escape: bool,
    /// The left mouse button is held down.
    pub left: bool,
}

/// Whether this build can pick a colour from the screen.
pub const fn available() -> bool {
    !cfg!(target_arch = "wasm32")
}

/// Whether the caller picks by polling [`poll_pixel`] (Windows) instead of
/// waiting for [`start_sampler`]'s result.
pub const fn polls_pixels() -> bool {
    cfg!(target_os = "windows")
}

/// Why the button is greyed out (empty when it works).
pub const fn unavailable_reason() -> &'static str {
    if available() {
        ""
    } else {
        "The browser's EyeDropper needs JavaScript; picking from the screen is not available in the web app"
    }
}

/// Colour component 0..=1 (as the system gives it) to a byte; non-finite counts as 0.
#[cfg_attr(any(target_os = "windows", target_arch = "wasm32"), allow(dead_code))]
pub fn unit_to_u8(v: f64) -> u8 {
    if !v.is_finite() {
        return 0;
    }
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Start the system's colour sampler (macOS, Linux/BSD). `wake` is called
/// (from any thread) when the result is in, so the UI can repaint; the result
/// arrives on the receiver. Errors when the sampler cannot start, or when this
/// platform has none (Windows: use [`poll_pixel`]; web).
pub fn start_sampler(wake: impl Fn() + Send + 'static) -> Result<Receiver<Outcome>, String> {
    platform_start(wake)
}

#[cfg(target_os = "windows")]
fn platform_start(_wake: impl Fn() + Send + 'static) -> Result<Receiver<Outcome>, String> {
    Err("Windows picks by polling the pixel under the pointer".into())
}

#[cfg(target_arch = "wasm32")]
fn platform_start(_wake: impl Fn() + Send + 'static) -> Result<Receiver<Outcome>, String> {
    Err(unavailable_reason().to_string())
}

#[cfg(target_os = "macos")]
fn platform_start(wake: impl Fn() + Send + 'static) -> Result<Receiver<Outcome>, String> {
    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSColor;
    if MainThreadMarker::new().is_none() {
        return Err("The colour sampler must start on the main thread".into());
    }
    let (tx, rx) = std::sync::mpsc::channel::<Outcome>();
    // The block must be `Fn`; the mutex makes the (Send, not Sync) channel and callback usable from it.
    let state = std::sync::Mutex::new((tx, wake));
    let handler = RcBlock::new(move |color: *mut NSColor| {
        let o = sampled_outcome(color);
        if let Ok(s) = state.lock() {
            // The window may have closed meanwhile: nobody to tell then.
            let _ = s.0.send(o);
            (s.1)();
        }
    });
    show_sampler(&handler);
    Ok(rx)
}

/// The block's colour as an outcome (nil is a cancel).
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn sampled_outcome(color: *mut objc2_app_kit::NSColor) -> Outcome {
    use objc2_app_kit::NSColorSpace;
    // SAFETY: AppKit passes nil or a valid NSColor, used only inside the handler call.
    match unsafe { color.as_ref() } {
        None => Outcome::Cancelled,
        Some(c) => {
            let srgb = NSColorSpace::sRGBColorSpace();
            match c.colorUsingColorSpace(&srgb) {
                Some(c) => Outcome::Picked([
                    unit_to_u8(c.redComponent()),
                    unit_to_u8(c.greenComponent()),
                    unit_to_u8(c.blueComponent()),
                ]),
                None => Outcome::Failed("The sampled colour has no sRGB value".into()),
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn show_sampler(handler: &block2::RcBlock<dyn Fn(*mut objc2_app_kit::NSColor)>) {
    use objc2_app_kit::NSColorSampler;
    let sampler = NSColorSampler::new();
    // SAFETY: only called on the main thread (checked by the caller); the sampler keeps itself
    // and the block alive until the selection is made.
    unsafe { sampler.showSamplerWithSelectionHandler(handler) };
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn platform_start(wake: impl Fn() + Send + 'static) -> Result<Receiver<Outcome>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<Outcome>();
    std::thread::Builder::new()
        .name("colour-pick".into())
        .spawn(move || {
            let r = pollster::block_on(async {
                let req = ashpd::desktop::Color::pick().send().await?;
                req.response()
            });
            let o = match r {
                Ok(c) => Outcome::Picked([unit_to_u8(c.red()), unit_to_u8(c.green()), unit_to_u8(c.blue())]),
                Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => Outcome::Cancelled,
                Err(ashpd::Error::PortalNotFound(_)) => Outcome::Failed(
                    "Picking from the screen needs xdg-desktop-portal with a Screenshot backend, which was not found"
                        .into(),
                ),
                Err(e) => Outcome::Failed(format!("Picking from the screen failed: {e}")),
            };
            // The window may have closed meanwhile: nobody to tell then.
            let _ = tx.send(o);
            wake();
        })
        .map(|_| rx)
        .map_err(|e| format!("Could not start the colour pick: {e}"))
}

/// Windows: the pixel under the pointer and the Escape and left-button state.
/// Elsewhere: all empty.
pub fn poll_pixel() -> PixelPoll {
    #[cfg(target_os = "windows")]
    {
        win::poll()
    }
    #[cfg(not(target_os = "windows"))]
    {
        PixelPoll::default()
    }
}

#[cfg(target_os = "windows")]
mod win {
    use super::PixelPoll;
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC, CLR_INVALID};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    #[allow(unsafe_code)]
    fn down(vk: u16) -> bool {
        // SAFETY: GetAsyncKeyState has no preconditions.
        (unsafe { GetAsyncKeyState(i32::from(vk)) } as u16 & 0x8000) != 0
    }

    #[allow(unsafe_code)]
    fn pixel_under_pointer() -> Option<[u8; 3]> {
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: plain Win32 calls with a valid out pointer; the DC is released.
        unsafe {
            if GetCursorPos(&mut p) == 0 {
                return None;
            }
            let dc = GetDC(std::ptr::null_mut());
            if dc.is_null() {
                return None;
            }
            let c = GetPixel(dc, p.x, p.y);
            ReleaseDC(std::ptr::null_mut(), dc);
            if c == CLR_INVALID {
                return None;
            }
            Some([(c & 0xff) as u8, ((c >> 8) & 0xff) as u8, ((c >> 16) & 0xff) as u8])
        }
    }

    pub fn poll() -> PixelPoll {
        PixelPoll {
            colour: pixel_under_pointer(),
            escape: down(VK_ESCAPE),
            left: down(VK_LBUTTON),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_conversion_clamps_and_rounds() {
        assert_eq!(unit_to_u8(0.0), 0);
        assert_eq!(unit_to_u8(1.0), 255);
        assert_eq!(unit_to_u8(0.5), 128);
        assert_eq!(unit_to_u8(-3.0), 0);
        assert_eq!(unit_to_u8(7.0), 255);
        assert_eq!(unit_to_u8(f64::NAN), 0);
        assert_eq!(unit_to_u8(f64::INFINITY), 0);
    }

    #[test]
    fn availability_and_reason_agree() {
        assert_eq!(available(), unavailable_reason().is_empty());
    }

    #[test]
    fn poll_is_empty_off_windows() {
        if !polls_pixels() {
            assert_eq!(poll_pixel(), PixelPoll::default());
        }
    }
}
