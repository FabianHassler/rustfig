//! Pick a colour from the screen (E4b): the egui side. The OS calls (macOS
//! `NSColorSampler`, Windows `GetPixel`, the Linux portal) are in
//! `rustfig-os`; this keeps the pick in progress and repaints.
//!
//! Windows has no system sampler: while picking, every frame reads the pixel
//! under the pointer, the picker shows it live, the next left click takes it
//! and Escape cancels. The web build has no pick: the button is greyed out.

use std::sync::mpsc::Receiver;

pub use rustfig_os::screen::{available, unavailable_reason, Outcome};

/// A pick in progress.
#[derive(Default)]
pub struct ScreenPicker {
    rx: Option<Receiver<Outcome>>,
    /// Windows: the colour under the pointer, shown live.
    live: Option<[u8; 3]>,
    /// Windows: picking is on.
    own: bool,
    /// Windows: the left button that started the pick has not been released yet.
    wait_release: bool,
}

impl std::fmt::Debug for ScreenPicker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenPicker").field("active", &self.active()).finish()
    }
}

impl ScreenPicker {
    /// A pick is waiting for the user.
    pub fn active(&self) -> bool {
        self.rx.is_some() || self.own
    }

    /// The colour under the pointer while picking (Windows only).
    pub fn live(&self) -> Option<[u8; 3]> {
        self.live
    }

    /// Hand a result in as if the system had delivered it (tests).
    #[cfg(test)]
    pub fn inject(&mut self, o: Outcome) {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(o);
        self.rx = Some(rx);
    }

    /// Start a pick. Returns an error text when it cannot start.
    pub fn start(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if self.active() {
            return Ok(());
        }
        if !available() {
            return Err(unavailable_reason().to_string());
        }
        if rustfig_os::screen::polls_pixels() {
            self.own = true;
            self.wait_release = true;
            self.live = None;
            return Ok(());
        }
        let ctx = ctx.clone();
        self.rx = Some(rustfig_os::screen::start_sampler(move || ctx.request_repaint())?);
        Ok(())
    }

    /// Called every frame: the result when the pick has ended.
    pub fn poll(&mut self, ctx: &egui::Context) -> Option<Outcome> {
        if self.own {
            return self.poll_own(ctx);
        }
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(o) => {
                self.rx = None;
                Some(o)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.rx = None;
                Some(Outcome::Failed("The colour sampler ended without a colour".into()))
            }
        }
    }

    /// Windows' own picking, one frame.
    fn poll_own(&mut self, ctx: &egui::Context) -> Option<Outcome> {
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
        let s = rustfig_os::screen::poll_pixel();
        self.live = s.colour;
        if s.escape {
            return Some(self.end_own(Outcome::Cancelled));
        }
        if self.wait_release {
            self.wait_release = s.left;
            return None;
        }
        if s.left {
            let o = match self.live {
                Some(c) => Outcome::Picked(c),
                None => Outcome::Failed("Could not read the screen".into()),
            };
            return Some(self.end_own(o));
        }
        None
    }

    fn end_own(&mut self, o: Outcome) -> Outcome {
        self.own = false;
        self.live = None;
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_delivers_a_result_once() {
        let ctx = egui::Context::default();
        let mut p = ScreenPicker::default();
        assert!(!p.active());
        assert_eq!(p.poll(&ctx), None);
        p.inject(Outcome::Picked([1, 2, 3]));
        assert!(p.active());
        assert_eq!(p.poll(&ctx), Some(Outcome::Picked([1, 2, 3])));
        assert!(!p.active());
        assert_eq!(p.poll(&ctx), None);
    }

    #[test]
    fn dead_sender_is_a_failure() {
        let ctx = egui::Context::default();
        let mut p = ScreenPicker::default();
        let (tx, rx) = std::sync::mpsc::channel();
        drop(tx);
        p.rx = Some(rx);
        assert!(matches!(p.poll(&ctx), Some(Outcome::Failed(_))));
        assert!(!p.active());
    }
}
