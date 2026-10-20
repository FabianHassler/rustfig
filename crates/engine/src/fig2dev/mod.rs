//! fig2dev in the engine (`fig2dev`, a program of its own that
//! RustFig depends on, plan B12):
//!
//! - Every export goes through fig2dev: the document is written as Fig 3.2
//!   (`rustfig_format`), read by fig2dev's reader and handed to the
//!   language's driver with fig2dev's options ([`export`]), as xfig hands
//!   a figure to fig2dev. A language is available as soon as
//!   `fig2dev::drivers::driver` provides it; the combined LaTeX
//!   exports (`pstex`, `pdftex`, `pspdftex`) need both halves.
//! - Fig files before 3.0 (1.3, 1.4, 2.0, 2.1, which RustFig's own reader
//!   doesn't read) open through fig2dev's reader, converted to a document
//!   by [`to_model`] (plan G1, S3).
//! - Pictures as fig2dev reads them (`readpics.c`), where the `image`
//!   crate can't.
//!
//! fig2dev's language and option tables are re-exported here
//! ([`languages`], [`options`]) for the export dialog and the CLI.

pub mod to_model;

pub use fig2dev::util::readpics::LoadedPicture;
pub use fig2dev::{languages, options};

use fig2dev::{read, Ctx};
use languages::Language;
use rustfig_model::Document;
use std::path::Path;

/// Whether `src` is a Fig file older than 3.0: `#FIG 1.x`/`#FIG 2.x`, or
/// Fig 1.3's header of numbers.
pub fn is_old_fig(src: &[u8]) -> bool {
    let start = src.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(src.len());
    let head = src.get(start..).unwrap_or_default();
    if head.starts_with(b"#FIG 1") || head.starts_with(b"#FIG 2") {
        return true;
    }
    // Fig 1.3 has no "#FIG" line: it starts with "ppi coord_sys w h".
    start == 0 && head.first().is_some_and(u8::is_ascii_digit)
}

/// Read a Fig file of any version fig2dev reads (1.3 to 3.2) with its
/// reader; the document and fig2dev's messages (its repairs).
pub fn read_old_fig(src: &[u8]) -> Result<(Document, Vec<String>), String> {
    let mut ctx = Ctx::default();
    match read::readfp_fig(&mut ctx, src) {
        Ok(objects) => {
            let doc = to_model::to_document(&ctx, &objects);
            Ok((doc, std::mem::take(&mut ctx.messages)))
        }
        Err(_) if ctx.messages.is_empty() => Err("not a Fig file fig2dev can read".into()),
        Err(_) => Err(ctx.messages.join("\n")),
    }
}

/// Whether the fig2dev crate writes language `id` (fig2dev's name).
pub fn driver_available(id: &str) -> bool {
    fig2dev::drivers::driver(id).is_some()
}

/// Whether RustFig can export `lang`: fig2dev's driver for it is ported
/// (for the combined languages, the drivers of both halves).
pub fn language_available(lang: &Language) -> bool {
    match lang.id {
        "pstex" => driver_available("pstex") && driver_available("pstex_t"),
        "pdftex" => driver_available("pdftex") && driver_available("pdftex_t"),
        "pspdftex" => driver_available("pstex") && driver_available("pdftex") && driver_available("pstex_t"),
        "pspdf" => driver_available("eps") && driver_available("pdf"),
        id => driver_available(id),
    }
}

/// Export `doc` with fig2dev's driver for `lang` (fig2dev's `-L` name):
/// `options` are fig2dev's option letters and arguments, in order (general
/// ones too); `dir` is where relative picture names are found. The output
/// and fig2dev's messages (its warnings); fig2dev's errors are `Err`.
pub fn export(
    doc: &Document,
    lang: &str,
    options: &[(char, Option<String>)],
    dir: Option<&Path>,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let mut dev = fig2dev::drivers::driver(lang).ok_or_else(|| format!("no fig2dev driver for {lang}"))?;
    let mut ctx = Ctx {
        lang: lang.to_string(),
        dir: dir.map(Path::to_path_buf),
        ..Ctx::default()
    };
    let l = Some(lang.to_string());
    for (c, arg) in std::iter::once(&('L', l)).chain(options) {
        let arg = arg.as_deref();
        if !ctx.general_option(*c, arg)? {
            dev.option(&mut ctx, *c, arg)?;
        }
    }
    // After the options, so that -m counts (fig2dev's get_args).
    ctx.font_size *= ctx.fontmag;
    let fig = rustfig_format::write_fig(doc);
    let objects = read::readfp_fig(&mut ctx, fig.as_bytes()).map_err(|_| ctx.messages.join("\n"))?;
    let status = fig2dev::convert(&mut ctx, dev.as_mut(), &objects)?;
    let messages = std::mem::take(&mut ctx.messages);
    if status.exit != 0 && status.output.is_empty() {
        return Err(if messages.is_empty() {
            format!("fig2dev's {lang} driver failed")
        } else {
            messages.join("\n")
        });
    }
    Ok((status.output, messages))
}

/// Read picture `path` as fig2dev does (`.gz`/`.Z` variants, XBM, XPM,
/// PCX, EPS, PDF and the common bitmaps).
pub fn load_picture(path: &Path) -> Result<LoadedPicture, String> {
    fig2dev::util::readpics::load_picture(&mut Ctx::default(), &path.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_versions_are_recognised() {
        assert!(is_old_fig(b"#FIG 2.1\n80 2\n"));
        assert!(is_old_fig(b"#FIG 1.4\n"));
        assert!(is_old_fig(b"1200 2 9 9\n"));
        assert!(!is_old_fig(b"#FIG 3.2\n"));
        assert!(!is_old_fig(b"{\"objects\": []}"));
        assert!(!is_old_fig(b""));
        assert!(!is_old_fig(b"hello"));
    }
}
