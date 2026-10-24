//! RustFig: the desktop application (and the web entry point on wasm).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    // fig2dev first: started under that name, or `rustfig fig2dev ARGS…`.
    // The GUI is not started and its option parsing never sees the arguments.
    if rustfig_engine::fig2dev::started_as_fig2dev() {
        std::process::exit(i32::from(rustfig_engine::fig2dev::run_cli(
            rustfig_engine::fig2dev::process_args(),
        )));
    }
    // (`args_os`, lossily: a file name that is not UTF-8 must not abort the start.)
    if rustfig_engine::fig2dev::process_args().first().map(String::as_str) == Some("fig2dev") {
        let mut args = rustfig_engine::fig2dev::process_args();
        args.drain(..1.min(args.len()));
        std::process::exit(i32::from(rustfig_engine::fig2dev::run_cli(args)));
    }
    // `--dump-icon SIZE`: the app icon as raw RGBA on stdout (used by `cargo xtask package`).
    {
        let a = rustfig_engine::fig2dev::process_args();
        if a.first().map(String::as_str) == Some("--dump-icon") {
            use std::io::Write;
            let size = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(256);
            let Some(px) = rustfig_ui_egui::app_icon::rgba(size) else {
                eprintln!("rustfig: could not render the icon");
                std::process::exit(1);
            };
            let ok = std::io::stdout().write_all(&px).is_ok();
            std::process::exit(i32::from(!ok));
        }
    }
    // xfig's command line (S3), plus RustFig's --control and --background.
    let opts = match rustfig_engine::cmdline::parse(rustfig_engine::fig2dev::process_args()) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            eprintln!("Try `rustfig -help`.");
            std::process::exit(2);
        }
    };
    if opts.help {
        eprintln!("rustfig {} (xfig's command line)", env!("CARGO_PKG_VERSION"));
        eprint!("{}", rustfig_engine::cmdline::usage());
        eprintln!("       rustfig fig2dev ARGS...   (fig2dev's command line; also when started as `fig2dev`)");
        return Ok(());
    }
    if opts.version {
        println!("rustfig {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    for n in &opts.notes {
        eprintln!("rustfig: {n}");
    }
    if let Some(files) = &opts.update {
        std::process::exit(update_files(files));
    }
    let control = opts.control;
    let background = opts.background;
    let file = opts.file.clone();
    let mut engine = rustfig_engine::Engine::new();
    let mut startup_error = None;
    if let Some(f) = &file {
        if let Err(e) = engine.execute("file.open", &serde_json::json!({ "path": f })) {
            startup_error = Some(e.message);
        }
    }
    let overrides = opts.override_table();
    let xdefaults = xdefaults_table();
    let (size, position) = overrides
        .get("geometry")
        .and_then(|g| g.as_str())
        .map_or((None, None), parse_geometry);
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(size.unwrap_or([1280.0, 860.0]))
        .with_min_inner_size([640.0, 480.0])
        .with_title("RustFig");
    if let Some(icon) = rustfig_ui_egui::app_icon::icon_data() {
        viewport = viewport.with_icon(icon);
    }
    if let Some(p) = position {
        viewport = viewport.with_position(p);
    }
    if background && std::env::var_os("RUSTFIG_RECOVERY_DIR").is_none() {
        // A background instance (an agent's or a script's) that gets killed
        // must not offer its drawings to the user's next start.
        std::env::set_var(
            "RUSTFIG_RECOVERY_DIR",
            std::env::temp_dir().join("rustfig-background-recovery"),
        );
    }
    if background {
        viewport = viewport
            .with_active(false)
            .with_window_level(egui::WindowLevel::AlwaysOnBottom);
    }
    #[allow(unused_mut)] // set on macOS only
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    if background {
        options.event_loop_builder = Some(Box::new(|b| {
            use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
            b.with_activation_policy(ActivationPolicy::Accessory)
                .with_activate_ignoring_other_apps(false);
        }));
    }
    // Finder double-click and `open -a` arrive as an Apple Event (P3); listen before the loop starts.
    rustfig_ui_egui::install_open_handler();
    eframe::run_native(
        "RustFig",
        options,
        Box::new(move |cc| {
            let mut app = rustfig_ui_egui::App::new(cc, engine, control);
            // xfig's options and X resources apply to this run only.
            rustfig_ui_egui::settings::start_session(&mut app, &xdefaults, &overrides);
            if let Some(e) = startup_error {
                app.error(e);
            }
            Ok(Box::new(app))
        }),
    )
}

/// `Fig.*` resources of `~/.Xdefaults` and `~/.Xresources` (lowest priority).
#[cfg(not(target_arch = "wasm32"))]
fn xdefaults_table() -> rustfig_engine::prefs::TomlTable {
    let Some(home) = std::env::var_os("HOME") else {
        return Default::default();
    };
    let mut pairs = Vec::new();
    for name in [".Xresources", ".Xdefaults"] {
        if let Ok(text) = std::fs::read_to_string(std::path::Path::new(&home).join(name)) {
            pairs.extend(rustfig_engine::cmdline::parse_resources(&text));
        }
    }
    rustfig_engine::cmdline::resource_table(&pairs)
}

/// `-geometry [WxH][+X+Y]`: the window size and position, when given.
#[cfg(not(target_arch = "wasm32"))]
fn parse_geometry(g: &str) -> (Option<[f32; 2]>, Option<[f32; 2]>) {
    let (size, pos) = match g.find(['+', '-']) {
        Some(i) => g.split_at(i),
        None => (g, ""),
    };
    let size = size.split_once(['x', 'X']).and_then(|(w, h)| {
        let (w, h) = (w.parse::<f32>().ok()?, h.parse::<f32>().ok()?);
        ((100.0..=20_000.0).contains(&w) && (100.0..=20_000.0).contains(&h)).then_some([w, h])
    });
    let position = pos.strip_prefix('+').and_then(|p| {
        let (x, y) = p.split_once('+')?;
        Some([x.parse::<f32>().ok()?, y.parse::<f32>().ok()?])
    });
    (size, position)
}

/// `xfig -update FILE...`: read each file and write it back in the current
/// Fig format, keeping the original as `FILE.bak`. Returns the exit status.
#[cfg(not(target_arch = "wasm32"))]
fn update_files(files: &[String]) -> i32 {
    if files.is_empty() {
        eprintln!("xfig: -update needs file names");
        return 2;
    }
    let mut failed = 0;
    for f in files {
        let mut engine = rustfig_engine::Engine::new();
        let r = std::fs::copy(f, format!("{f}.bak"))
            .map_err(|e| format!("{f}.bak: {e}"))
            .and_then(|_| {
                engine
                    .execute("file.open", &serde_json::json!({ "path": f }))
                    .map_err(|e| e.message)
            })
            .and_then(|_| {
                engine
                    .execute("file.save", &serde_json::json!({ "path": f }))
                    .map_err(|e| e.message)
            });
        match r {
            Ok(_) => eprintln!("{f}: updated (original kept as {f}.bak)"),
            Err(e) => {
                eprintln!("{f}: {e}");
                failed += 1;
            }
        }
    }
    i32::from(failed > 0)
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;
    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(canvas) = document
            .get_element_by_id("rustfig_canvas")
            .and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else {
            return;
        };
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    Ok(Box::new(rustfig_ui_egui::App::new(
                        cc,
                        rustfig_engine::Engine::new(),
                        None,
                    )))
                }),
            )
            .await;
        // The page's "Loading" text goes once the app runs; a start-up error
        // is shown by eframe in the page.
        if started.is_ok() {
            if let Some(l) = document.get_element_by_id("loading") {
                l.remove();
            }
        }
    });
}
