//! RustFig desktop and web UI (layer L6), built on egui.
//!
//! The UI is thin: panels read engine state and act through
//! [`App::run`], which executes engine commands or the UI-only commands in
//! [`menus::UI_COMMANDS`].

pub mod app_icon;
pub mod attributes;
#[cfg(not(target_arch = "wasm32"))]
pub mod autosave;
pub mod canvas;
#[cfg(not(target_arch = "wasm32"))]
pub mod control;
pub mod dialogs;
pub mod menus;
pub mod mode_icons;
pub mod panels;
pub mod rulers;
pub mod settings;
pub mod theme;
pub mod vector_icons;
pub mod xfig_icons;

use rustfig_engine::{Engine, UiRequest};
use serde_json::{json, Value};

/// Most messages kept in the message panel's history.
pub const MAX_MESSAGES: usize = 200;

/// Window layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// xfig's arrangement: menus, file name, message panel, mouse indicator and
    /// units box at the top; depth panel on the right.
    Classic,
    /// Menu bar at the top, status bar at the bottom.
    Modern,
}

/// A message shown in the message panel / status bar.
#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
    pub error: bool,
}

/// What to do once the user has dealt with unsaved changes.
#[derive(Debug, Clone, PartialEq)]
pub enum Pending {
    New,
    Open(Option<String>),
    Quit,
}

/// The application.
pub struct App {
    pub engine: Engine,
    pub canvas: canvas::CanvasState,
    pub dialogs: dialogs::Dialogs,
    /// The latest message.
    pub message: Option<Message>,
    /// Message history, oldest first.
    pub messages: Vec<Message>,
    pub layout: Layout,
    pub show_depths: bool,
    /// xfig's "Show info balloons" (Meta-Y): tooltips on or off.
    pub show_balloons: bool,
    /// Attribute pictures drawn as vectors at the screen's density (W9);
    /// off shows xfig's 1-bit bitmaps.
    pub vector_icons: bool,
    /// xfig's mode menu popped up by Alt+wheel over the canvas: drawing
    /// (wheel up) or editing modes, at the pointer.
    pub mode_popup: Option<(bool, egui::Pos2)>,
    /// Crash-recovery copies of the drawing (A6); `None` until enabled.
    #[cfg(not(target_arch = "wasm32"))]
    autosave: Option<autosave::AutoSave>,
    /// Drawings left by instances that stopped without saving, offered at
    /// start: (recovery id, the file they came from).
    pub recoverable: Vec<(String, Option<String>)>,
    /// How long the last frames took on the UI thread (ms), newest last,
    /// at most 120 (native; `ui.state` reports them).
    pub frame_times: std::collections::VecDeque<f64>,
    /// Where the preferences are kept (E1); nowhere until `App::new` or a
    /// test sets one.
    pub settings: settings::Settings,
    /// xfig's compose key while typing text (`menus::compose_text`).
    pub(crate) compose: menus::Compose,
    pub show_indicator: bool,
    /// While dragging across depth checkboxes: the state they are set to.
    depth_drag: Option<bool>,
    /// Mouse hints shown instead of the tool's while the pointer is over a
    /// panel with its own button functions (the attribute buttons).
    pub hint_override: Option<(&'static str, &'static str, &'static str)>,
    /// xfig's keyboard accelerators (Meta menu keys, Ctrl mode keys).
    pub xfig_keys: bool,
    /// A classic menu to pop up on the next frame (xfig's Meta-F/E/V/H).
    pub open_menu: Option<&'static str>,
    /// The Unit/Scale settings window (right click on the units box).
    pub unit_panel_open: bool,
    /// The override set during the previous frame, shown in this one.
    pub hint_shown: Option<(&'static str, &'static str, &'static str)>,
    pub pending: Option<Pending>,
    /// File pickers (native dialogs; replaced in tests).
    pub file_dialogs: Box<dyn dialogs::FileDialogs>,
    quit_confirmed: bool,
    #[cfg(not(target_arch = "wasm32"))]
    control: Option<control::Server>,
    #[cfg(not(target_arch = "wasm32"))]
    screenshot_waiters: Vec<(String, std::sync::mpsc::Sender<String>, Value)>,
}

impl App {
    /// Create the app without eframe (used by `new` and by UI tests).
    pub fn with_engine(engine: Engine) -> App {
        App {
            engine,
            canvas: canvas::CanvasState::default(),
            dialogs: dialogs::Dialogs::default(),
            message: None,
            messages: Vec::new(),
            layout: Layout::Classic,
            show_depths: true,
            show_balloons: true,
            vector_icons: true,
            mode_popup: None,
            #[cfg(not(target_arch = "wasm32"))]
            autosave: None,
            recoverable: Vec::new(),
            frame_times: std::collections::VecDeque::new(),
            settings: settings::Settings::default(),
            compose: menus::Compose::default(),
            show_indicator: true,
            depth_drag: None,
            hint_override: None,
            hint_shown: None,
            unit_panel_open: false,
            xfig_keys: true,
            open_menu: None,
            pending: None,
            file_dialogs: Box::new(dialogs::NativeDialogs),
            quit_confirmed: false,
            #[cfg(not(target_arch = "wasm32"))]
            control: None,
            #[cfg(not(target_arch = "wasm32"))]
            screenshot_waiters: Vec::new(),
        }
    }

    /// Create the app. `control_port` starts the JSON-lines control server (native only).
    pub fn new(cc: &eframe::CreationContext<'_>, engine: Engine, control_port: Option<u16>) -> App {
        cc.egui_ctx.set_theme(egui::ThemePreference::System);
        // Mutated only by the native control server below.
        #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
        let mut app = App::with_engine(engine);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(port) = control_port {
            match control::Server::start(port, cc.egui_ctx.clone()) {
                Ok(s) => {
                    log::info!("control channel listening on 127.0.0.1:{port}");
                    app.control = Some(s);
                }
                Err(e) => {
                    log::error!("control channel: failed to bind 127.0.0.1:{port}: {e}");
                    app.error(format!("Control port {port} unavailable: {e}"));
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = control_port;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(dir) = rustfig_engine::recovery::default_dir() {
            app.enable_recovery(dir);
        }
        app.settings = settings::Settings::platform();
        settings::load_and_apply(&mut app);
        app
    }

    /// Keep crash-recovery copies in `dir` and offer the drawings that
    /// stopped instances left there (A6).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_recovery(&mut self, dir: std::path::PathBuf) {
        self.engine.recovery_dir = Some(dir.clone());
        self.autosave = Some(autosave::AutoSave::new(dir));
        if let Ok(v) = self.engine.execute("file.recoverable", &serde_json::json!({})) {
            self.recoverable = v
                .get("drawings")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|d| {
                    let id = d.get("id")?.as_str()?.to_string();
                    let original = d
                        .get("original")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string);
                    Some((id, original))
                })
                .collect();
        }
    }

    /// Whether the classic xfig layout is active.
    pub fn classic(&self) -> bool {
        self.layout == Layout::Classic
    }

    fn push_message(&mut self, m: Message) {
        self.messages.push(m.clone());
        if self.messages.len() > MAX_MESSAGES {
            let extra = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(..extra);
        }
        self.message = Some(m);
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.push_message(Message {
            text: text.into(),
            error: false,
        });
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push_message(Message {
            text: text.into(),
            error: true,
        });
    }

    /// Run a command (engine or UI-only). Errors are shown in the status bar.
    pub fn run(&mut self, id: &str, params: Value) -> Option<Value> {
        match self.try_run(id, &params) {
            Ok(v) => Some(v),
            Err(e) => {
                self.error(e);
                None
            }
        }
    }

    /// Run a command and return its result or error message.
    pub fn try_run(&mut self, id: &str, params: &Value) -> Result<Value, String> {
        if menus::find_ui(id).is_some() {
            return menus::run_ui(self, id, params);
        }
        let r = self.engine.execute(id, params).map_err(|e| e.message);
        if r.is_ok() && !self.engine.status().is_empty() {
            let s = self.engine.status().to_string();
            self.info(s);
            self.engine.set_status("");
        }
        r
    }

    /// Whether a command is currently enabled.
    pub fn enabled(&self, id: &str) -> bool {
        if let Some(c) = rustfig_engine::cmd::find(id) {
            return (c.enabled)(&self.engine);
        }
        menus::find_ui(id).is_some()
    }

    fn handle_requests(&mut self) {
        for r in self.engine.take_requests() {
            match r {
                UiRequest::EditObject { id } => self.dialogs.open_edit(&self.engine, id),
                UiRequest::PickPicture { rect } => {
                    if let Some(path) = self.file_dialogs.pick_file("Pictures", dialogs::PICTURE_EXTS) {
                        self.run(
                            "shape.picture",
                            json!({"path": path, "x": rect.min.x, "y": rect.min.y, "width": rect.width(), "height": rect.height()}),
                        );
                    }
                }
            }
        }
    }

    /// Ask before discarding unsaved changes.
    /// Whether quitting was confirmed (the window closes on the next frame).
    pub fn quit_requested(&self) -> bool {
        self.quit_confirmed
    }

    pub fn guard_unsaved(&mut self, action: Pending) {
        if self.engine.is_modified() {
            self.pending = Some(action);
        } else {
            self.do_pending(action);
        }
    }

    pub(crate) fn do_pending(&mut self, action: Pending) {
        match action {
            Pending::New => {
                self.run("file.new", json!({}));
                self.engine.set_modified(false);
            }
            Pending::Open(path) => {
                let path = path.or_else(|| self.file_dialogs.pick_file("Figures", dialogs::FIGURE_EXTS));
                if let Some(p) = path {
                    self.run("file.open", json!({ "path": p }));
                }
            }
            Pending::Quit => {
                self.quit_confirmed = true;
            }
        }
    }

    fn window_title(&self) -> String {
        let name = self
            .engine
            .path()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");
        format!(
            "{}{} — RustFig",
            name,
            if self.engine.is_modified() { " •" } else { "" }
        )
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Unsaved work the user didn't discard keeps its recovery copy.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(a) = &mut self.autosave {
            if self.quit_confirmed || !self.engine.is_modified() {
                a.finish();
            }
        }
    }
}

impl App {
    /// Run one UI frame (guarded against panics).
    pub fn ui(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        let frame_result = rustfig_engine::guard::catch(|| self.frame(ctx));
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.frame_times.len() >= 120 {
                self.frame_times.pop_front();
            }
            self.frame_times.push_back(start.elapsed().as_secs_f64() * 1000.0);
        }
        if let Err(msg) = frame_result {
            // xfig's emergency save: keep the work before reporting.
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(a) = &mut self.autosave {
                a.emergency(&self.engine);
            }
            self.error(format!("internal error (please report): {msg}"));
        }
    }

    fn frame(&mut self, ctx: &egui::Context) {
        apply_balloons(ctx, self.show_balloons);
        #[cfg(not(target_arch = "wasm32"))]
        self.poll_control(ctx);
        let tokens = theme::Tokens::for_dark(ctx.style().visuals.dark_mode);
        // Panels set the hint override while hovered; show last frame's.
        self.hint_shown = self.hint_override.take();
        menus::handle_shortcuts(self, ctx);
        if self.classic() {
            egui::TopBottomPanel::top("xfig_top").show(ctx, |ui| panels::classic_top(self, ui, &tokens));
        } else {
            egui::TopBottomPanel::top("menu").show(ctx, |ui| menus::menu_bar(self, ui));
            egui::TopBottomPanel::bottom("status").show(ctx, |ui| panels::status_bar(self, ui, &tokens));
        }
        if self.show_indicator {
            egui::TopBottomPanel::bottom("indicator").show(ctx, |ui| attributes::attribute_panel(self, ui, &tokens));
        }
        egui::SidePanel::left("modes")
            .resizable(false)
            .exact_width(84.0)
            .show(ctx, |ui| panels::mode_panel(self, ui, &tokens));
        if self.show_depths {
            egui::SidePanel::right("depths")
                .default_width(140.0)
                .show(ctx, |ui| panels::depth_panel(self, ui, &tokens));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| canvas::show(self, ui, &tokens));
        dialogs::show(self, ctx);
        panels::mode_popup(self, ctx, &tokens);
        self.handle_requests();
        settings::tick(self);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(a) = &mut self.autosave {
            if let Some(wait) = a.tick(&self.engine, std::time::Instant::now()) {
                ctx.request_repaint_after(wait);
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));
        if ctx.input(|i| i.viewport().close_requested()) && !self.quit_confirmed && self.engine.is_modified() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Quit);
        }
        if self.quit_confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn poll_control(&mut self, ctx: &egui::Context) {
        // Screenshots arrive as events a frame or two after the request.
        let shots: Vec<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
            i.raw
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
                .collect()
        });
        if let Some(img) = shots.last() {
            for (path, reply, id) in self.screenshot_waiters.drain(..) {
                let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_srgba_unmultiplied()).collect();
                let res = rustfig_render::raster::encode_rgba_png(img.size[0] as u32, img.size[1] as u32, &rgba)
                    .and_then(|png| std::fs::write(&path, png).map_err(|e| e.to_string()));
                let msg = match res {
                    Ok(()) => json!({"id": id, "result": {"path": path, "width": img.size[0], "height": img.size[1]}}),
                    Err(e) => json!({"id": id, "error": {"code": "io", "message": e}}),
                };
                let _ = reply.send(msg.to_string()); // the client may have gone away
            }
        }
        let reqs = match &self.control {
            Some(s) => s.drain(),
            None => return,
        };
        for r in reqs {
            if r.method == "ui.screenshot" {
                let path = r
                    .params
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("rustfig-screenshot.png")
                    .to_string();
                self.screenshot_waiters.push((path, r.reply, r.id));
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                ctx.request_repaint();
                continue;
            }
            let res = control::handle(self, &r.method, &r.params);
            let msg = match res {
                Ok(v) => json!({"id": r.id, "result": v}),
                Err((code, m)) => json!({"id": r.id, "error": {"code": code, "message": m}}),
            };
            let _ = r.reply.send(msg.to_string()); // the client may have gone away
            ctx.request_repaint();
        }
    }
}

/// Tooltip delay with balloons on (egui's default) and a delay that never
/// passes with them off.
const BALLOON_DELAY: f32 = 0.5;
const NO_BALLOONS: f32 = 1.0e9;

fn apply_balloons(ctx: &egui::Context, on: bool) {
    let want = if on { BALLOON_DELAY } else { NO_BALLOONS };
    if ctx.style().interaction.tooltip_delay != want {
        ctx.style_mut(|s| s.interaction.tooltip_delay = want);
    }
}
