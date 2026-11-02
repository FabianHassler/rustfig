//! rustfig-quicklook: the executable of the macOS Quick Look app extensions (thumbnails and
//! previews of Fig files). `cargo xtask package` copies it into `RustFig.app/Contents/PlugIns/`
//! as `RustFigThumbnail.appex` and `RustFigPreview.appex`; the system starts it, and it hands
//! over to `NSExtensionMain` (see `rustfig_os::quicklook`).

use std::process::ExitCode;

fn main() -> ExitCode {
    // Text in thumbnails must not depend on the fonts of the system or the sandbox.
    rustfig_text_builtin();
    let code = rustfig_os::quicklook::run_extension(rustfig_engine::thumbnail::make_png);
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}

/// Built-in fonts: the extension's sandbox may not read every font folder.
fn rustfig_text_builtin() {
    std::env::set_var("FIG2DEV_BUILTIN_FONTS", "1");
}
