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
    if std::env::args().nth(1).as_deref() == Some("fig2dev") {
        let mut args = rustfig_engine::fig2dev::process_args();
        args.drain(..1.min(args.len()));
        std::process::exit(i32::from(rustfig_engine::fig2dev::run_cli(args)));
    }
    let mut args = std::env::args().skip(1);
    let mut control = None;
    let mut file = None;
    let mut background = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--control" => control = args.next().and_then(|p| p.parse::<u16>().ok()),
            // For agents and scripts: open behind other windows, without
            // taking the focus (macOS: no Dock icon either).
            "--background" => background = true,
            "--help" | "-h" => {
                println!("usage: rustfig [--control PORT] [--background] [FILE.fig|FILE.rustfig]\n       rustfig fig2dev ARGS...   (fig2dev's command line; also when started as `fig2dev`)");
                return Ok(());
            }
            "--version" | "-V" => {
                println!("rustfig {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            other => file = Some(other.to_string()),
        }
    }
    let mut engine = rustfig_engine::Engine::new();
    let mut startup_error = None;
    if let Some(f) = &file {
        if let Err(e) = engine.execute("file.open", &serde_json::json!({ "path": f })) {
            startup_error = Some(e.message);
        }
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 860.0])
        .with_min_inner_size([640.0, 480.0])
        .with_title("RustFig");
    if let Some(icon) = rustfig_ui_egui::app_icon::icon_data() {
        viewport = viewport.with_icon(icon);
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
    eframe::run_native(
        "RustFig",
        options,
        Box::new(move |cc| {
            let mut app = rustfig_ui_egui::App::new(cc, engine, control);
            if let Some(e) = startup_error {
                app.error(e);
            }
            Ok(Box::new(app))
        }),
    )
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
        let _ = eframe::WebRunner::new()
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
            .await; // start-up errors are shown by eframe in the page
    });
}
