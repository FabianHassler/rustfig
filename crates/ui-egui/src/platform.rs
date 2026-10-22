//! What differs between the operating systems in the texts the user reads:
//! the mouse buttons on a two-button mouse or trackpad, the modifier keys'
//! names, and the parts of the manual written for one system.
//!
//! The system is egui's (`Context::os`): fixed at compile time in the
//! native builds, read from the browser on the web.

/// The systems whose texts differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Mac,
    Windows,
    Linux,
}

impl Os {
    /// The system RustFig runs on.
    pub fn current(ctx: &egui::Context) -> Os {
        Os::from_egui(ctx.os())
    }

    pub fn from_egui(os: egui::os::OperatingSystem) -> Os {
        use egui::os::OperatingSystem as O;
        match os {
            O::Mac | O::IOS => Os::Mac,
            O::Windows => Os::Windows,
            _ => Os::Linux,
        }
    }

    /// The name used in the manual's markers (`<!-- os: mac -->`).
    fn tag(self) -> &'static str {
        match self {
            Os::Mac => "mac",
            Os::Windows => "windows",
            Os::Linux => "linux",
        }
    }

    /// How the three xfig buttons are reached with fewer buttons.
    pub fn mouse_note(self) -> &'static str {
        match self {
            Os::Mac => {
                "Trackpad or one-button mouse: Option+click is the middle button, \
                 Ctrl+click (or a two-finger click) the right button."
            }
            Os::Windows | Os::Linux => {
                "Two-button mouse: press the left and right buttons together for the middle \
                 button, or use Alt+click."
            }
        }
    }
}

/// The manual text for `os`: a block between `<!-- os: NAME … -->` and
/// `<!-- os: end -->` stays only when `os` is one of the names; the marker
/// lines themselves go. On GitHub the markers are invisible comments, so
/// readers there see every system's text.
pub fn for_os(text: &str, os: Os) -> String {
    let mut out = String::with_capacity(text.len());
    let mut keep = true;
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("<!-- os:").and_then(|r| r.strip_suffix("-->")) {
            let names: Vec<&str> = rest.split_whitespace().collect();
            keep = names == ["end"] || names.contains(&os.tag());
            continue;
        }
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_follow_the_system() {
        let src = "a\n<!-- os: mac -->\nmac only\n<!-- os: linux windows -->\npc only\n<!-- os: end -->\nb\n";
        assert_eq!(for_os(src, Os::Mac), "a\nmac only\nb\n");
        assert_eq!(for_os(src, Os::Linux), "a\npc only\nb\n");
        assert_eq!(for_os(src, Os::Windows), "a\npc only\nb\n");
        assert_eq!(for_os("plain\n", Os::Mac), "plain\n");
    }

    #[test]
    fn every_system_has_a_mouse_note() {
        for os in [Os::Mac, Os::Windows, Os::Linux] {
            assert!(!os.mouse_note().is_empty());
        }
        assert!(Os::Mac.mouse_note().contains("Option"));
        assert!(!Os::Linux.mouse_note().contains("Option"));
    }
}
