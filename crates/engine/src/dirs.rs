//! Where RustFig keeps its files on each system (P2): the settings in the
//! config folder, the recovery copies and saved versions in the data folder.
//! The rules are functions of the system and of the environment, so the
//! tests check all three systems on any one of them.
//!
//! - macOS: `~/Library/Application Support/RustFig` (both).
//! - Windows: `%APPDATA%\RustFig` (settings, roaming) and
//!   `%LOCALAPPDATA%\RustFig` (recovery, local to the machine); without those
//!   variables `%USERPROFILE%\AppData\Roaming` and `…\Local`.
//! - Linux and the BSDs: `$XDG_CONFIG_HOME/rustfig` (`~/.config/rustfig`) and
//!   `$XDG_DATA_HOME/rustfig` (`~/.local/share/rustfig`); a relative XDG value is
//!   ignored, as the XDG specification says.
//!
//! The paths may hold any Unicode; they stay `PathBuf`s (never forced through
//! `str`) wherever they are made and used.

use std::path::{Path, PathBuf};

/// The systems whose folders differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Mac,
    Windows,
    Linux,
}

impl HostOs {
    /// The system RustFig was built for.
    pub const fn current() -> HostOs {
        if cfg!(target_os = "macos") {
            HostOs::Mac
        } else if cfg!(windows) {
            HostOs::Windows
        } else {
            HostOs::Linux
        }
    }
}

/// A variable of the process environment that is set and not empty.
pub fn process_env(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// The folder for settings.
pub fn config_dir(os: HostOs, env: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    match os {
        HostOs::Mac => env("HOME").map(|h| h.join("Library/Application Support/RustFig")),
        HostOs::Windows => env("APPDATA")
            .or_else(|| env("USERPROFILE").map(|u| u.join("AppData").join("Roaming")))
            .map(|d| d.join("RustFig")),
        HostOs::Linux => xdg(env, "XDG_CONFIG_HOME", ".config").map(|d| d.join("rustfig")),
    }
}

/// The folder for data that stays on the machine (recovery copies).
pub fn data_dir(os: HostOs, env: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    match os {
        HostOs::Mac => env("HOME").map(|h| h.join("Library/Application Support/RustFig")),
        HostOs::Windows => env("LOCALAPPDATA")
            .or_else(|| env("USERPROFILE").map(|u| u.join("AppData").join("Local")))
            .map(|d| d.join("RustFig")),
        HostOs::Linux => xdg(env, "XDG_DATA_HOME", ".local/share").map(|d| d.join("rustfig")),
    }
}

fn xdg(env: &dyn Fn(&str) -> Option<PathBuf>, var: &str, home_sub: &str) -> Option<PathBuf> {
    env(var)
        // XDG wants an absolute path; `has_root`, not `is_absolute`, so the
        // Linux rules give the same answer when tested on Windows (no drive).
        .filter(|p| Path::new(p).has_root())
        .or_else(|| env("HOME").map(|h| h.join(home_sub)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<PathBuf> {
        let m: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |k| m.get(k).filter(|v| !v.is_empty()).map(PathBuf::from)
    }

    #[test]
    fn macos_uses_application_support() {
        let e = env(&[("HOME", "/Users/Jürgen Müller")]);
        let want = PathBuf::from("/Users/Jürgen Müller/Library/Application Support/RustFig");
        assert_eq!(config_dir(HostOs::Mac, &e), Some(want.clone()));
        assert_eq!(data_dir(HostOs::Mac, &e), Some(want));
        assert_eq!(config_dir(HostOs::Mac, &env(&[])), None);
    }

    #[test]
    fn windows_uses_roaming_for_settings_and_local_for_recovery() {
        let e = env(&[
            ("APPDATA", r"C:\Users\Zoë\AppData\Roaming"),
            ("LOCALAPPDATA", r"C:\Users\Zoë\AppData\Local"),
        ]);
        assert_eq!(
            config_dir(HostOs::Windows, &e),
            Some(PathBuf::from(r"C:\Users\Zoë\AppData\Roaming").join("RustFig"))
        );
        assert_eq!(
            data_dir(HostOs::Windows, &e),
            Some(PathBuf::from(r"C:\Users\Zoë\AppData\Local").join("RustFig"))
        );
        // Without the variables: under the profile.
        let e = env(&[("USERPROFILE", r"C:\Users\Zoë")]);
        let c = config_dir(HostOs::Windows, &e).unwrap();
        assert!(
            c.ends_with(Path::new("AppData").join("Roaming").join("RustFig")),
            "{c:?}"
        );
        let d = data_dir(HostOs::Windows, &e).unwrap();
        assert!(d.ends_with(Path::new("AppData").join("Local").join("RustFig")), "{d:?}");
        assert_eq!(config_dir(HostOs::Windows, &env(&[])), None);
    }

    #[test]
    fn linux_follows_xdg_and_ignores_relative_values() {
        let e = env(&[("HOME", "/home/anna")]);
        assert_eq!(config_dir(HostOs::Linux, &e), Some("/home/anna/.config/rustfig".into()));
        assert_eq!(
            data_dir(HostOs::Linux, &e),
            Some("/home/anna/.local/share/rustfig".into())
        );
        let e = env(&[
            ("HOME", "/home/anna"),
            ("XDG_CONFIG_HOME", "/cfg"),
            ("XDG_DATA_HOME", "/data"),
        ]);
        assert_eq!(config_dir(HostOs::Linux, &e), Some("/cfg/rustfig".into()));
        assert_eq!(data_dir(HostOs::Linux, &e), Some("/data/rustfig".into()));
        let e = env(&[
            ("HOME", "/home/anna"),
            ("XDG_CONFIG_HOME", "relative/dir"),
            ("XDG_DATA_HOME", ""),
        ]);
        assert_eq!(config_dir(HostOs::Linux, &e), Some("/home/anna/.config/rustfig".into()));
        assert_eq!(
            data_dir(HostOs::Linux, &e),
            Some("/home/anna/.local/share/rustfig".into())
        );
        assert_eq!(config_dir(HostOs::Linux, &env(&[])), None);
    }
}
