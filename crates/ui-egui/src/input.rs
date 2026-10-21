//! Platform conventions applied to the raw input before a frame.
//!
//! For mice and trackpads without three buttons, everywhere in the app
//! (canvas, rulers, panels, menus):
//! - Option/Alt + left button is the middle button (all platforms);
//! - macOS: Ctrl + left button is the right button, the system's own
//!   convention (the OS reports it to apps as a left click with Ctrl held).
//!
//! Neither modifier is used with the left button otherwise.
//!
//! Linux and Windows: left and right pressed together are the middle button
//! (X11's "emulate 3 buttons"), for when a window manager takes Alt + click.

use egui::{Event, Modifiers, PointerButton, Pos2, RawInput};

/// How close together left and right must go down to count as the middle
/// button (seconds). The first press waits this long before it counts as
/// itself.
pub const CHORD_INTERVAL: f64 = 0.05;

/// Left + right pressed together as the middle button.
#[derive(Debug, Default)]
pub struct Chord {
    /// A left or right press held back while the other may follow.
    pending: Option<(PointerButton, Pos2, Modifiers, f64)>,
    /// Buttons of an active chord still down (the middle release comes when
    /// both are up).
    chord_down: u8,
}

impl Chord {
    /// Whether a press is held back (the caller repaints so it gets out).
    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }

    pub fn apply(&mut self, raw: &mut RawInput) {
        let Some(now) = raw.time else {
            // No clock: no chords, nothing held back.
            let mut held = Vec::new();
            self.flush_into(&mut held);
            raw.events.splice(0..0, held);
            return;
        };
        let mut out = Vec::with_capacity(raw.events.len() + 1);
        for ev in std::mem::take(&mut raw.events) {
            match ev {
                Event::PointerButton {
                    pos,
                    button: b @ (PointerButton::Primary | PointerButton::Secondary),
                    pressed,
                    modifiers,
                } => {
                    if self.chord_down > 0 {
                        // Inside a chord: presses are swallowed, the middle
                        // button goes up with the last of the two.
                        if !pressed {
                            self.chord_down -= 1;
                            if self.chord_down == 0 {
                                out.push(Event::PointerButton {
                                    pos,
                                    button: PointerButton::Middle,
                                    pressed: false,
                                    modifiers,
                                });
                            }
                        }
                        continue;
                    }
                    match (self.pending, pressed) {
                        (Some((pb, ppos, pmods, t)), true) if pb != b && now - t <= CHORD_INTERVAL => {
                            self.pending = None;
                            self.chord_down = 2;
                            out.push(Event::PointerButton {
                                pos: ppos,
                                button: PointerButton::Middle,
                                pressed: true,
                                modifiers: pmods,
                            });
                        }
                        (None, true) => self.pending = Some((b, pos, modifiers, now)),
                        _ => {
                            self.flush_into(&mut out);
                            out.push(Event::PointerButton {
                                pos,
                                button: b,
                                pressed,
                                modifiers,
                            });
                        }
                    }
                }
                Event::PointerMoved(_) | Event::PointerButton { .. } | Event::PointerGone => {
                    self.flush_into(&mut out);
                    out.push(ev);
                }
                other => out.push(other),
            }
        }
        if self.pending.is_some_and(|(_, _, _, t)| now - t > CHORD_INTERVAL) {
            self.flush_into(&mut out);
        }
        raw.events = out;
    }

    /// Let a held-back press out as itself.
    fn flush_into(&mut self, out: &mut Vec<Event>) {
        if let Some((button, pos, modifiers, _)) = self.pending.take() {
            out.push(Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers,
            });
        }
    }
}

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

    fn at(time: f64, events: Vec<Event>) -> RawInput {
        RawInput {
            time: Some(time),
            events,
            ..RawInput::default()
        }
    }

    fn press(b: PointerButton, pressed: bool) -> Event {
        Event::PointerButton {
            pos: pos2(1.0, 1.0),
            button: b,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    fn pressed(raw: &RawInput) -> Vec<(PointerButton, bool)> {
        raw.events
            .iter()
            .filter_map(|e| match e {
                Event::PointerButton { button, pressed, .. } => Some((*button, *pressed)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn left_and_right_together_are_the_middle_button() {
        use PointerButton::{Middle, Primary, Secondary};
        for (first, second) in [(Primary, Secondary), (Secondary, Primary)] {
            let mut c = Chord::default();
            let a = at(1.0, vec![press(first, true)]);
            let mut a = a;
            c.apply(&mut a);
            assert!(pressed(&a).is_empty() && c.waiting());
            let mut b = at(1.02, vec![press(second, true)]);
            c.apply(&mut b);
            assert_eq!(pressed(&b), [(Middle, true)]);
            let mut up1 = at(1.2, vec![press(first, false)]);
            c.apply(&mut up1);
            assert!(pressed(&up1).is_empty());
            let mut up2 = at(1.25, vec![press(second, false)]);
            c.apply(&mut up2);
            assert_eq!(pressed(&up2), [(Middle, false)]);
        }
    }

    #[test]
    fn a_single_click_stays_itself() {
        use PointerButton::Primary;
        let mut c = Chord::default();
        // Press and release in one frame: out together, in order.
        let mut quick = at(1.0, vec![press(Primary, true), press(Primary, false)]);
        c.apply(&mut quick);
        assert_eq!(pressed(&quick), [(Primary, true), (Primary, false)]);
        // A held press comes out once the interval has passed.
        let mut a = at(2.0, vec![press(Primary, true)]);
        c.apply(&mut a);
        assert!(pressed(&a).is_empty());
        let mut later = at(2.0 + CHORD_INTERVAL + 0.01, vec![]);
        c.apply(&mut later);
        assert_eq!(pressed(&later), [(Primary, true)]);
        assert!(!c.waiting());
        // Without a clock nothing is held back.
        let mut no_clock = RawInput {
            events: vec![press(Primary, true)],
            ..RawInput::default()
        };
        c.apply(&mut no_clock);
        assert_eq!(pressed(&no_clock).len(), 1);
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
