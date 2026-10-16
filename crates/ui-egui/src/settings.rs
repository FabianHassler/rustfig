//! Where the preferences live (E1) and the parts the UI applies: the
//! settings file `settings.toml` in the platform's config folder (native)
//! or the browser's storage (web). The engine applies the drawing-related
//! resources (`rustfig_engine::prefs`); this module applies the panels,
//! balloons, layout, keymap, mouse buttons and export panel, and writes the
//! file whenever a setting changes.

use crate::{App, Layout};
use rustfig_engine::prefs::Prefs;
use rustfig_engine::MouseScheme;

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
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            store: Store::None,
            saved: None,
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
        Settings { store, saved: None }
    }

    /// Settings kept in `path` (native).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn file(path: std::path::PathBuf) -> Settings {
        Settings {
            store: Store::File(path),
            saved: None,
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

    /// Write `prefs` unless they are what was last written.
    pub fn save(&mut self, prefs: &Prefs) -> Result<(), String> {
        if matches!(self.store, Store::None) {
            return Ok(());
        }
        let text = prefs.to_toml();
        if self.saved.as_deref() == Some(text.as_str()) {
            return Ok(());
        }
        self.write(&text)?;
        self.saved = Some(text);
        Ok(())
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
    if let Some(m) = p.int("export_margin") {
        // xfig's margin is in points.
        app.dialogs.export.margin = (m.clamp(0, 10_000) as f64) * 1200.0 / 72.0;
    }
    let r = &p.rustfig;
    if let Some(l) = r.layout.as_deref() {
        app.layout = if l == "modern" { Layout::Modern } else { Layout::Classic };
    }
    if let Some(m) = r.mouse.as_deref() {
        app.engine.set_mouse_scheme(if m == "modern" {
            MouseScheme::Modern
        } else {
            MouseScheme::Xfig
        });
    }
    if let Some(b) = r.xfig_keys {
        app.xfig_keys = b;
    }
    if let Some(b) = r.show_attribute_panel {
        app.show_indicator = b;
    }
    if let Some(b) = r.vector_icons {
        app.vector_icons = b;
    }
}

/// Write the current switches into the preferences (they persist).
pub fn collect_ui(app: &mut App) {
    let layout = Some(if app.classic() { "classic" } else { "modern" }.to_string());
    let mouse = Some(
        if app.engine.mouse_scheme() == MouseScheme::Modern {
            "modern"
        } else {
            "xfig"
        }
        .to_string(),
    );
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
    upd(&mut r.layout, layout);
    upd(&mut r.mouse, mouse);
    for (slot, v) in [
        (&mut r.xfig_keys, app.xfig_keys),
        (&mut r.show_attribute_panel, app.show_indicator),
        (&mut r.vector_icons, app.vector_icons),
    ] {
        if *slot != Some(v) {
            *slot = Some(v);
            changed = true;
        }
    }
    // xfig's own switches in Global settings and the View menu.
    for (name, v) in [
        ("showballoons", balloons),
        ("showdepthmanager", depths),
        ("showpageborder", border),
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
    if let Some(p) = prefs {
        app.engine.prefs = p;
    }
    let prefs = app.engine.prefs.clone();
    for w in prefs.apply(&mut app.engine) {
        app.error(format!("settings: {w}"));
    }
    apply_ui(app);
    app.engine.prefs_dirty = false;
}
