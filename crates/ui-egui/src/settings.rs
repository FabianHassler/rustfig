//! Where the preferences live (E1) and the parts the UI applies: the
//! settings file `settings.toml` in the platform's config folder (native)
//! or the browser's storage (web). The engine applies the drawing-related
//! resources (`rustfig_engine::prefs`); this module applies the panels,
//! balloons, layout, keymap, mouse buttons and export panel, and writes the
//! file whenever a setting changes.

use crate::App;
use rustfig_engine::prefs::{Prefs, TomlTable, TomlValue};

/// The browser storage key (web).
#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "rustfig.settings";

enum Store {
    /// Nothing is read or written (tests, or no config folder).
    None,
    #[cfg(not(target_arch = "wasm32"))]
    File(std::path::PathBuf),
    #[cfg(target_arch = "wasm32")]
    Browser,
}

/// The settings store and what was last written to it.
pub struct Settings {
    store: Store,
    saved: Option<String>,
    /// Values set for this run only (command line, `~/.Xdefaults`): key, the
    /// run's value and what the stored settings had (S3). Not written back
    /// while the setting still has the run's value.
    session: Vec<(String, TomlValue, Option<TomlValue>)>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            store: Store::None,
            saved: None,
            session: Vec::new(),
        }
    }
}

impl Settings {
    /// The platform's store: the settings file (native) or browser storage.
    pub fn platform() -> Settings {
        #[cfg(not(target_arch = "wasm32"))]
        let store = rustfig_engine::prefs::config_path().map_or(Store::None, Store::File);
        #[cfg(target_arch = "wasm32")]
        let store = Store::Browser;
        Settings {
            store,
            saved: None,
            session: Vec::new(),
        }
    }

    /// Settings kept in `path` (native).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn file(path: std::path::PathBuf) -> Settings {
        Settings {
            store: Store::File(path),
            saved: None,
            session: Vec::new(),
        }
    }

    /// Where the settings are kept, for messages.
    pub fn location(&self) -> String {
        match &self.store {
            Store::None => "nowhere".into(),
            #[cfg(not(target_arch = "wasm32"))]
            Store::File(p) => p.display().to_string(),
            #[cfg(target_arch = "wasm32")]
            Store::Browser => "the browser's storage".into(),
        }
    }

    fn read(&self) -> Option<String> {
        match &self.store {
            Store::None => None,
            #[cfg(not(target_arch = "wasm32"))]
            Store::File(p) => std::fs::read_to_string(p).ok(),
            #[cfg(target_arch = "wasm32")]
            Store::Browser => web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten()),
        }
    }

    fn write(&self, text: &str) -> Result<(), String> {
        match &self.store {
            Store::None => Ok(()),
            #[cfg(not(target_arch = "wasm32"))]
            Store::File(p) => {
                if let Some(dir) = p.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                }
                let tmp = p.with_extension("toml.tmp");
                std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
                std::fs::rename(&tmp, p).map_err(|e| format!("{}: {e}", p.display()))
            }
            #[cfg(target_arch = "wasm32")]
            Store::Browser => web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .ok_or_else(|| "no browser storage".to_string())?
                .set_item(STORAGE_KEY, text)
                .map_err(|_| "the browser refused to store the settings".to_string()),
        }
    }

    /// Read the stored preferences: `None` when there are none yet; a
    /// damaged file gives the defaults and a message.
    pub fn load(&mut self) -> (Option<Prefs>, Vec<String>) {
        let Some(text) = self.read() else {
            return (None, Vec::new());
        };
        match Prefs::parse(&text) {
            Ok((p, warnings)) => {
                self.saved = Some(p.to_toml());
                (Some(p), warnings)
            }
            Err(e) => (None, vec![format!("{e} ({}); using xfig's defaults", self.location())]),
        }
    }

    /// `prefs` with this run's values replaced by the stored ones.
    fn without_session(&self, prefs: &Prefs) -> Prefs {
        let mut p = prefs.clone();
        for (k, v, stored) in &self.session {
            if p.xfig.get(k) == Some(v) {
                match stored {
                    Some(o) => p.xfig.insert(k.clone(), o.clone()),
                    None => p.xfig.remove(k),
                };
            }
        }
        p
    }

    /// Write `prefs` unless they are what was last written.
    pub fn save(&mut self, prefs: &Prefs) -> Result<(), String> {
        if matches!(self.store, Store::None) {
            return Ok(());
        }
        let text = self.without_session(prefs).to_toml();
        if self.saved.as_deref() == Some(text.as_str()) {
            return Ok(());
        }
        self.write(&text)?;
        self.saved = Some(text);
        Ok(())
    }
}

/// The UI scale limits (E9): 75 % to 200 %; not finite means 100 %.
pub fn clamp_scale(s: f32) -> f32 {
    if s.is_finite() {
        s.clamp(0.75, 2.0)
    } else {
        1.0
    }
}

/// Apply the UI's resources and switches from the engine's preferences.
pub fn apply_ui(app: &mut App) {
    let p = app.engine.prefs.clone();
    if let Some(b) = p.bool("showballoons") {
        app.show_balloons = b;
    }
    if let Some(b) = p.bool("showdepthmanager") {
        app.show_depths = b;
    }
    if let Some(b) = p.bool("autorefresh") {
        app.reload.set(b);
    }
    if let Some(l) = p.str("exportLanguage") {
        match rustfig_engine::fig2dev::languages::language(&l)
            .filter(|l| rustfig_engine::fig2dev::language_available(l))
        {
            Some(lang) => app.dialogs.export.format = lang.id,
            None => app.info(format!(
                "exportLanguage: {l} can't be exported yet; using {}",
                app.dialogs.export.format
            )),
        }
    }
    let r = &p.rustfig;
    if let Some(b) = r.xfig_keys {
        app.xfig_keys = b;
    }
    if let Some(style) = r.button_style.as_deref() {
        app.button_3d = style != "flat";
    }
    if let Some(t) = r.theme.as_deref() {
        let _ = crate::menus::set_theme(app, t);
    }
    if let Some(s) = r.ui_scale {
        app.ui_scale = clamp_scale(s);
    }
}

/// Write the current switches into the preferences (they persist).
pub fn collect_ui(app: &mut App) {
    let (balloons, depths, border) = (app.show_balloons, app.show_depths, app.engine.show_page_border);
    let p = &mut app.engine.prefs;
    let r = &mut p.rustfig;
    let mut changed = false;
    let mut upd = |slot: &mut Option<String>, v: Option<String>| {
        if *slot != v {
            *slot = v;
            changed = true;
        }
    };
    upd(
        &mut r.button_style,
        Some(if app.button_3d { "3d" } else { "flat" }.to_string()),
    );
    upd(&mut r.theme, Some(app.theme.clone()));
    if r.ui_scale != Some(app.ui_scale) {
        r.ui_scale = Some(app.ui_scale);
        changed = true;
    }
    if r.xfig_keys != Some(app.xfig_keys) {
        r.xfig_keys = Some(app.xfig_keys);
        changed = true;
    }
    // xfig's own switches in Global settings and the View menu.
    for (name, v) in [
        ("showballoons", balloons),
        ("showdepthmanager", depths),
        ("showpageborder", border),
        ("autorefresh", app.reload.on),
    ] {
        if p.bool(name) != Some(v) && p.set(name, toml_bool(v)).is_ok() {
            changed = true;
        }
    }
    if changed {
        app.engine.prefs_dirty = true;
    }
}

fn toml_bool(b: bool) -> rustfig_engine::prefs::TomlValue {
    rustfig_engine::prefs::TomlValue::Boolean(b)
}

/// Once per frame: note the switches; write the file if anything changed.
pub fn tick(app: &mut App) {
    collect_ui(app);
    if app.engine.prefs_dirty {
        app.engine.prefs_dirty = false;
        let prefs = app.engine.prefs.clone();
        if let Err(e) = app.settings.save(&prefs) {
            app.error(format!("Couldn't save the settings: {e}"));
        }
    }
}

/// Read the stored preferences and apply them (at start).
pub fn load_and_apply(app: &mut App) {
    let (prefs, messages) = app.settings.load();
    for m in messages {
        app.error(m);
    }
    // A settings file that does not exist yet (not the test store).
    #[cfg(not(target_arch = "wasm32"))]
    let first_start = prefs.is_none() && matches!(&app.settings.store, Store::File(p) if !p.exists());
    #[cfg(target_arch = "wasm32")]
    let first_start = false;
    if let Some(p) = prefs {
        app.engine.prefs = p;
    }
    // First start: take xfig's recent files from `~/.xfigrc` (S3).
    #[cfg(not(target_arch = "wasm32"))]
    if first_start {
        if let Some(text) = std::env::var_os("HOME")
            .and_then(|h| std::fs::read_to_string(std::path::Path::new(&h).join(".xfigrc")).ok())
        {
            let n = app.engine.prefs.import_xfigrc(&text);
            if n > 0 {
                app.info(format!("Took {n} recent files from xfig's ~/.xfigrc"));
            }
        }
    }
    let imported = first_start && !app.engine.prefs.rustfig.recent_files.is_empty();
    let prefs = app.engine.prefs.clone();
    for w in prefs.apply(&mut app.engine) {
        app.error(format!("settings: {w}"));
    }
    apply_ui(app);
    // Saved once, so the import happens on the first start only.
    app.engine.prefs_dirty = imported;
    crate::help_pages::check_version(app);
}

/// Values for this run only (S3): `defaults` (X resource files) fill in
/// resources the stored settings don't have; `overrides` (the command
/// line) win over everything. Neither is written to the settings file.
pub fn start_session(app: &mut App, defaults: &TomlTable, overrides: &TomlTable) {
    let mut entries = Vec::new();
    for (k, v) in defaults {
        if !app.engine.prefs.xfig.contains_key(k) {
            entries.push((k.clone(), v.clone()));
        }
    }
    entries.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
    for (k, v) in entries {
        let stored = app.engine.prefs.xfig.get(&k).cloned();
        let stored = app
            .settings
            .session
            .iter()
            .find(|s| s.0 == k)
            .map_or(stored, |s| s.2.clone());
        app.settings.session.retain(|s| s.0 != k);
        app.engine.prefs.xfig.insert(k.clone(), v.clone());
        app.settings.session.push((k, v, stored));
    }
    let prefs = app.engine.prefs.clone();
    for w in prefs.apply(&mut app.engine) {
        app.error(format!("options: {w}"));
    }
    apply_ui(app);
}
