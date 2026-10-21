//! Injected mouse and keyboard input (control channel `ui.pointer`, `ui.key`,
//! `ui.click_label`) and the per-frame registry of labelled widgets.
//!
//! Input is queued as steps, one step per frame, and added to egui's raw
//! input in `App::raw_input_hook` before the platform conventions
//! (`input::CtrlClick`) run, so it behaves exactly like real events.
//! Parameters are untrusted: everything is checked and capped.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Most steps waiting at once.
pub const MAX_STEPS: usize = 10_000;
/// Longest text accepted by `ui.key`, in characters.
pub const MAX_TEXT: usize = 1000;
/// Most widgets kept in the registry.
pub const MAX_WIDGETS: usize = 5000;
/// Empty frames after the last step, so its effects are drawn before the reply.
const SETTLE: usize = 3;
/// Largest coordinate or scroll amount accepted, in points.
const MAX_COORD: f32 = 100_000.0;

/// One frame's worth of injected input.
#[derive(Debug, Default, Clone)]
pub struct Step {
    pub events: Vec<Event>,
    /// Held modifiers during this frame (`None`: leave the real ones).
    pub modifiers: Option<Modifiers>,
}

/// The queue of steps and how many have been consumed.
#[derive(Debug, Default)]
pub struct Injector {
    queue: VecDeque<Step>,
    pushed: u64,
    consumed: u64,
}

impl Injector {
    /// Queue steps (and the settling frames); returns the sequence number
    /// the queue reaches when they have all been fed. Fails when full.
    pub fn push(&mut self, steps: Vec<Step>) -> Result<u64, String> {
        if self.queue.len() + steps.len() + SETTLE > MAX_STEPS {
            return Err("too much input queued".into());
        }
        self.pushed += (steps.len() + SETTLE) as u64;
        self.queue.extend(steps);
        self.queue.extend((0..SETTLE).map(|_| Step::default()));
        Ok(self.pushed)
    }

    pub fn is_idle(&self) -> bool {
        self.queue.is_empty()
    }

    /// Whether every step up to `seq` has been fed to egui.
    pub fn reached(&self, seq: u64) -> bool {
        self.consumed >= seq
    }

    /// Feed the next step into this frame's raw input.
    pub fn feed(&mut self, raw: &mut RawInput) {
        let Some(step) = self.queue.pop_front() else { return };
        self.consumed += 1;
        if let Some(m) = step.modifiers {
            raw.modifiers = m;
        }
        raw.events.extend(step.events);
    }
}

fn num(p: &Value, key: &str) -> Result<Option<f32>, String> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_f64() {
            Some(f) if f.is_finite() && f.abs() <= f64::from(MAX_COORD) => Ok(Some(f as f32)),
            _ => Err(format!("{key} must be a finite number within ±{MAX_COORD}")),
        },
    }
}

pub fn parse_button(p: &Value) -> Result<PointerButton, String> {
    match p.get("button").and_then(Value::as_str).unwrap_or("left") {
        "left" | "primary" => Ok(PointerButton::Primary),
        "middle" => Ok(PointerButton::Middle),
        "right" | "secondary" => Ok(PointerButton::Secondary),
        other => Err(format!("unknown button {other:?} (left, middle, right)")),
    }
}

pub fn parse_modifiers(p: &Value) -> Result<Modifiers, String> {
    let mut m = Modifiers::NONE;
    let list: Vec<&str> = match p.get("modifiers") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
        Some(Value::String(s)) => s.split(['+', ',']).collect(),
        Some(_) => return Err("modifiers must be an array of names".into()),
    };
    for name in list {
        match name.trim().to_ascii_lowercase().as_str() {
            "" => {}
            "shift" => m.shift = true,
            "ctrl" | "control" => m.ctrl = true,
            "alt" | "option" => m.alt = true,
            "command" | "cmd" | "meta" | "super" => {
                m.command = true;
                m.mac_cmd = cfg!(target_os = "macos");
                // Off macOS, command is Ctrl.
                m.ctrl |= !cfg!(target_os = "macos");
            }
            other => return Err(format!("unknown modifier {other:?} (shift, ctrl, alt, command)")),
        }
    }
    Ok(m)
}

fn button_event(pos: Pos2, button: PointerButton, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers,
    }
}

/// Steps for `ui.pointer`; `at` is the pointer position to use when the
/// request has no coordinates (the last one injected).
pub fn pointer_steps(p: &Value) -> Result<Vec<Step>, String> {
    let action = p
        .get("action")
        .or_else(|| p.get("type"))
        .and_then(Value::as_str)
        .ok_or("action is required (move, press, release, click, double_click, scroll)")?;
    let x = num(p, "x")?.ok_or("x is required")?;
    let y = num(p, "y")?.ok_or("y is required")?;
    let pos = Pos2::new(x, y);
    let button = parse_button(p)?;
    let m = parse_modifiers(p)?;
    let step = |events: Vec<Event>| Step {
        events,
        modifiers: Some(m),
    };
    let mv = || step(vec![Event::PointerMoved(pos)]);
    let click = |steps: &mut Vec<Step>| {
        steps.push(step(vec![button_event(pos, button, true, m)]));
        steps.push(step(vec![button_event(pos, button, false, m)]));
    };
    let mut steps = Vec::new();
    match action {
        "move" => steps.push(mv()),
        "press" => {
            steps.push(mv());
            steps.push(step(vec![button_event(pos, button, true, m)]));
        }
        "release" => {
            steps.push(mv());
            steps.push(step(vec![button_event(pos, button, false, m)]));
        }
        "click" => {
            steps.push(mv());
            click(&mut steps);
        }
        "double_click" => {
            steps.push(mv());
            click(&mut steps);
            click(&mut steps);
        }
        "scroll" => {
            let dx = num(p, "dx")?.unwrap_or(0.0);
            let dy = num(p, "dy")?.unwrap_or(0.0);
            steps.push(mv());
            steps.push(step(vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(dx, dy),
                modifiers: m,
            }]));
        }
        other => return Err(format!("unknown action {other:?}")),
    }
    Ok(steps)
}

/// Steps for `ui.key`: `key` (egui's key names: "F", "Enter", "Escape",
/// "ArrowDown", ...) pressed and released with `modifiers`, and/or `text`
/// typed.
pub fn key_steps(p: &Value) -> Result<Vec<Step>, String> {
    let m = parse_modifiers(p)?;
    let mut steps = Vec::new();
    if let Some(name) = p.get("key").and_then(Value::as_str) {
        let key = Key::from_name(name).ok_or_else(|| format!("unknown key {name:?}"))?;
        for pressed in [true, false] {
            steps.push(Step {
                events: vec![Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed,
                    repeat: false,
                    modifiers: m,
                }],
                modifiers: Some(m),
            });
        }
    }
    if let Some(text) = p.get("text").and_then(Value::as_str) {
        if text.chars().count() > MAX_TEXT {
            return Err(format!("text is longer than {MAX_TEXT} characters"));
        }
        // One character per frame, like typing.
        for c in text.chars().filter(|c| !c.is_control()) {
            steps.push(Step {
                events: vec![Event::Text(c.to_string())],
                modifiers: Some(m),
            });
        }
    }
    if steps.is_empty() {
        return Err("key or text is required".into());
    }
    Ok(steps)
}

/// A widget with an accessible label, from egui's accessibility tree.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetInfo {
    pub label: String,
    pub role: String,
    /// Left, top, right, bottom in points.
    pub rect: [f32; 4],
    pub disabled: bool,
}

impl WidgetInfo {
    pub fn center(&self) -> Pos2 {
        Pos2::new((self.rect[0] + self.rect[2]) / 2.0, (self.rect[1] + self.rect[3]) / 2.0)
    }
}

pub type Registry = Arc<Mutex<Vec<WidgetInfo>>>;

/// Egui plugin copying the accessibility tree of each pass into the registry.
struct RegistryPlugin(Registry);

impl egui::Plugin for RegistryPlugin {
    fn debug_name(&self) -> &'static str {
        "rustfig-widget-registry"
    }

    fn output_hook(&mut self, output: &mut egui::FullOutput) {
        let Some(update) = &output.platform_output.accesskit_update else {
            return;
        };
        let mut list = Vec::new();
        for (_, node) in &update.nodes {
            let label = node.label().or_else(|| node.value()).unwrap_or("");
            let Some(b) = node.bounds() else { continue };
            if label.is_empty() || node.is_hidden() || list.len() >= MAX_WIDGETS {
                continue;
            }
            list.push(WidgetInfo {
                label: label.to_string(),
                role: format!("{:?}", node.role()),
                rect: [b.x0 as f32, b.y0 as f32, b.x1 as f32, b.y1 as f32],
                disabled: node.is_disabled(),
            });
        }
        if let Ok(mut r) = self.0.lock() {
            *r = list;
        }
    }
}

/// Start collecting labelled widgets (turns accessibility output on).
pub fn install_registry(ctx: &egui::Context) -> Registry {
    let reg = Registry::default();
    ctx.enable_accesskit();
    ctx.add_plugin(RegistryPlugin(reg.clone()));
    reg
}

/// Find a widget by label: exact match first, then substring; ignores
/// widgets without area. `index` picks among several matches.
pub fn find_label(list: &[WidgetInfo], label: &str, index: usize) -> Option<WidgetInfo> {
    let visible = |w: &&WidgetInfo| w.rect[2] > w.rect[0] && w.rect[3] > w.rect[1] && w.role != "Window";
    let exact: Vec<&WidgetInfo> = list.iter().filter(visible).filter(|w| w.label == label).collect();
    if let Some(w) = exact.get(index) {
        return Some((*w).clone());
    }
    let part: Vec<&WidgetInfo> = list
        .iter()
        .filter(visible)
        .filter(|w| w.label.contains(label))
        .collect();
    part.get(index).map(|w| (*w).clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hostile_params_are_errors() {
        for p in [
            json!({}),
            json!({"action":"click"}),
            json!({"action":"click","x":1e300,"y":0}),
            json!({"action":"click","x":"a","y":0}),
            json!({"action":"nope","x":0,"y":0}),
            json!({"action":"click","x":0,"y":0,"button":"x"}),
            json!({"action":"click","x":0,"y":0,"modifiers":[1]}),
            json!({"action":"click","x":0,"y":0,"modifiers":5}),
        ] {
            let r = pointer_steps(&p);
            if p.get("modifiers") == Some(&json!([1])) {
                // Non-string entries are ignored, not fatal.
                assert!(r.is_ok());
            } else {
                assert!(r.is_err(), "{p}");
            }
        }
        assert!(key_steps(&json!({})).is_err());
        assert!(key_steps(&json!({"key":"Bogus"})).is_err());
        assert!(key_steps(&json!({"text":"x".repeat(MAX_TEXT + 1)})).is_err());
    }

    #[test]
    fn click_is_move_press_release_then_settling() {
        let mut inj = Injector::default();
        let steps = pointer_steps(&json!({"action":"click","x":5,"y":6,"modifiers":["shift"]})).unwrap();
        assert_eq!(steps.len(), 3);
        let seq = inj.push(steps).unwrap();
        let mut raw = RawInput::default();
        for i in 0..seq {
            assert!(!inj.reached(seq));
            raw.events.clear();
            inj.feed(&mut raw);
            if i == 0 {
                assert!(matches!(raw.events[0], Event::PointerMoved(_)));
                assert!(raw.modifiers.shift);
            }
        }
        assert!(inj.reached(seq) && inj.is_idle());
    }

    #[test]
    fn queue_is_capped() {
        let mut inj = Injector::default();
        let many = vec![Step::default(); MAX_STEPS];
        assert!(inj.push(many).is_err());
    }
}
