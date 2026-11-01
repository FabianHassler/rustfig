//! The message history (W10-29): messages carry a level (info, warning,
//! error), the history window filters by level and copies what is listed.
//! xfig's message window (Dismiss, Clear; `w_msgpanel.c`) is kept; the level
//! check boxes and the Copy button sit next to its buttons. Scripts use the
//! UI commands `app.messages` and `app.copy_messages`.

use crate::Message;
use serde_json::{json, Value};

/// How serious a message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Level {
    #[default]
    Info,
    Warning,
    Error,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Info, Level::Warning, Level::Error];

    pub fn name(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }

    pub fn from_name(s: &str) -> Option<Level> {
        Level::ALL.into_iter().find(|l| l.name() == s)
    }
}

/// Which levels the history window lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelFilter {
    pub info: bool,
    pub warning: bool,
    pub error: bool,
}

impl Default for LevelFilter {
    fn default() -> Self {
        LevelFilter {
            info: true,
            warning: true,
            error: true,
        }
    }
}

impl LevelFilter {
    pub fn shows(&self, l: Level) -> bool {
        match l {
            Level::Info => self.info,
            Level::Warning => self.warning,
            Level::Error => self.error,
        }
    }

    pub fn set(&mut self, l: Level, on: bool) {
        match l {
            Level::Info => self.info = on,
            Level::Warning => self.warning = on,
            Level::Error => self.error = on,
        }
    }

    /// Only the named levels (`["error", "warning"]`); unknown names are an error.
    pub fn from_names(names: &[Value]) -> Result<LevelFilter, String> {
        let mut f = LevelFilter {
            info: false,
            warning: false,
            error: false,
        };
        for n in names {
            let n = n.as_str().ok_or("\"levels\" must be strings")?;
            let l = Level::from_name(n).ok_or_else(|| format!("unknown level \"{n}\" (info, warning, error)"))?;
            f.set(l, true);
        }
        Ok(f)
    }
}

/// The messages the filter lets through, oldest first.
pub fn filtered<'a>(all: &'a [Message], f: &LevelFilter) -> Vec<&'a Message> {
    all.iter().filter(|m| f.shows(m.level)).collect()
}

/// The listed messages as text, one per line, as the Copy button copies them.
/// Warnings and errors carry their level as a prefix so the text still says
/// what it was after pasting.
pub fn as_text(msgs: &[&Message]) -> String {
    msgs.iter()
        .map(|m| match m.level {
            Level::Info => m.text.clone(),
            l => format!("{}: {}", l.name(), m.text),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn filter_from(app: &crate::App, p: &Value) -> Result<LevelFilter, String> {
    match p.get("levels").and_then(Value::as_array) {
        Some(a) => LevelFilter::from_names(a),
        None => Ok(app.dialogs.message_levels),
    }
}

/// `app.messages`: open the history window; returns the listed messages.
pub fn run_messages(app: &mut crate::App, p: &Value) -> Result<Value, String> {
    let f = filter_from(app, p)?;
    if p.get("levels").is_some() {
        app.dialogs.message_levels = f;
    }
    app.dialogs.messages_open = true;
    let list = filtered(&app.messages, &f);
    Ok(json!({"messages": list.iter().map(|m| json!({"level": m.level.name(), "text": m.text})).collect::<Vec<_>>()}))
}

/// `app.copy_messages`: the listed messages go to the clipboard on the next frame.
pub fn run_copy(app: &mut crate::App, p: &Value) -> Result<Value, String> {
    let f = filter_from(app, p)?;
    let text = as_text(&filtered(&app.messages, &f));
    let n = text.lines().count();
    app.dialogs.messages_copy = Some(text.clone());
    Ok(json!({"copied": n, "text": text}))
}

/// Hand a pending copy to the system clipboard.
pub fn flush_copy(app: &mut crate::App, ctx: &egui::Context) {
    if let Some(t) = app.dialogs.messages_copy.take() {
        ctx.copy_text(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(t: &str, level: Level) -> Message {
        Message { text: t.into(), level }
    }

    #[test]
    fn filter_and_text() {
        let all = vec![m("a", Level::Info), m("b", Level::Warning), m("c", Level::Error)];
        let mut f = LevelFilter::default();
        assert_eq!(filtered(&all, &f).len(), 3);
        f.info = false;
        let l = filtered(&all, &f);
        assert_eq!(as_text(&l), "warning: b\nerror: c");
        f.warning = false;
        assert_eq!(as_text(&filtered(&all, &f)), "error: c");
        assert!(LevelFilter::from_names(&[json!("bogus")]).is_err());
    }
}
