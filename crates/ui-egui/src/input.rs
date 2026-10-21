//! Platform conventions applied to the raw input before a frame.
//!
//! For mice and trackpads without three buttons, everywhere in the app
//! (canvas, rulers, panels, menus):
//! - Option/Alt + left button is the middle button (all platforms);
//! - macOS: Ctrl + left button is the right button, the system's own
//!   convention (the OS reports it to apps as a left click with Ctrl held).
//!
//! Neither modifier is used with the left button otherwise.

use egui::{Event, PointerButton, RawInput};

/// Turns Alt + left into the middle button and (with `ctrl_is_right`) Ctrl +
/// left into the right button; the release follows its press even if the
/// modifier was let go in between.
#[derive(Debug, Default)]
pub struct CtrlClick {
    down: Option<PointerButton>,
}

impl CtrlClick {
    pub fn apply(&mut self, raw: &mut RawInput, ctrl_is_right: bool) {
        for ev in &mut raw.events {
            if let Event::PointerButton {
                button,
                pressed,
                modifiers,
                ..
            } = ev
            {
                if *button != PointerButton::Primary {
                    continue;
                }
                let as_button = if *pressed {
                    self.down = if modifiers.alt {
                        Some(PointerButton::Middle)
                    } else if ctrl_is_right && modifiers.ctrl && !modifiers.command {
                        Some(PointerButton::Secondary)
                    } else {
                        None
                    };
                    self.down
                } else {
                    self.down.take()
                };
                if let Some(b) = as_button {
                    *button = b;
                    modifiers.ctrl = false;
                    modifiers.alt = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, Modifiers};

    fn button(pressed: bool, ctrl: bool) -> Event {
        modifier(pressed, ctrl, false)
    }

    fn modifier(pressed: bool, ctrl: bool, alt: bool) -> Event {
        let mut modifiers = Modifiers::NONE;
        modifiers.ctrl = ctrl;
        modifiers.alt = alt;
        Event::PointerButton {
            pos: pos2(1.0, 1.0),
            button: PointerButton::Primary,
            pressed,
            modifiers,
        }
    }

    fn buttons(raw: &RawInput) -> Vec<PointerButton> {
        raw.events
            .iter()
            .filter_map(|e| match e {
                Event::PointerButton { button, .. } => Some(*button),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn ctrl_click_becomes_a_right_click_and_its_release_follows() {
        let mut c = CtrlClick::default();
        let mut raw = RawInput {
            events: vec![button(true, true)],
            ..RawInput::default()
        };
        c.apply(&mut raw, true);
        // Ctrl let go before the button.
        let mut up = RawInput {
            events: vec![button(false, false)],
            ..RawInput::default()
        };
        c.apply(&mut up, true);
        assert_eq!(buttons(&raw), [PointerButton::Secondary]);
        assert_eq!(buttons(&up), [PointerButton::Secondary]);
        let mut plain = RawInput {
            events: vec![button(true, false), button(false, false)],
            ..RawInput::default()
        };
        c.apply(&mut plain, true);
        assert_eq!(buttons(&plain), [PointerButton::Primary, PointerButton::Primary]);
        // Without the macOS convention Ctrl + left stays left.
        let mut other = RawInput {
            events: vec![button(true, true), button(false, true)],
            ..RawInput::default()
        };
        c.apply(&mut other, false);
        assert_eq!(buttons(&other), [PointerButton::Primary, PointerButton::Primary]);
    }

    #[test]
    fn alt_click_is_the_middle_button() {
        let mut c = CtrlClick::default();
        let mut raw = RawInput {
            events: vec![modifier(true, false, true), modifier(false, false, false)],
            ..RawInput::default()
        };
        c.apply(&mut raw, false);
        assert_eq!(buttons(&raw), [PointerButton::Middle, PointerButton::Middle]);
    }
}
