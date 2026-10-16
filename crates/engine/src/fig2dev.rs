//! fig2dev's reader and drivers in the engine (`rustfig-fig2dev`):
//!
//! - Fig files before 3.0 (1.3, 1.4, 2.0, 2.1, which RustFig's own reader
//!   doesn't read) open through fig2dev's reader, converted to a document
//!   by `to_model` (plan G1, S3).
//! - The languages whose fig2dev driver is ported export through it: the
//!   document is written as Fig 3.2, read by fig2dev's reader and handed
//!   to the driver with fig2dev's options, as xfig hands a figure to
//!   fig2dev. Availability follows the crate: a language is available as
//!   soon as `rustfig_fig2dev::drivers::driver` provides it.
//! - The combined LaTeX exports (`pstex`, `pdftex`, `pspdftex`): the
//!   graphics through RustFig's EPS or PDF renderer without special texts,
//!   the LaTeX part through the `pstex_t` driver, as xfig runs fig2dev
//!   (`u_print.c`).
//! - Pictures as fig2dev reads them (`readpics.c`), where the `image`
//!   crate can't.

use rustfig_fig2dev::to_model::{read_picture, LoadedPicture};
use rustfig_fig2dev::{read, Ctx};
use rustfig_model::Document;
use rustfig_render::languages::Language;
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
            let doc = rustfig_fig2dev::to_model::to_document(&ctx, &objects);
            Ok((doc, std::mem::take(&mut ctx.messages)))
        }
        Err(_) if ctx.messages.is_empty() => Err("not a Fig file fig2dev can read".into()),
        Err(_) => Err(ctx.messages.join("\n")),
    }
}

/// Whether the fig2dev crate writes language `id` (fig2dev's name).
pub fn driver_available(id: &str) -> bool {
    rustfig_fig2dev::drivers::driver(id).is_some()
}

/// Whether RustFig can export `lang`: with its own renderers, through a
/// ported fig2dev driver, or (the combined LaTeX languages) both.
pub fn language_available(lang: &Language) -> bool {
    match lang.id {
        "pstex" | "pdftex" | "pspdftex" => driver_available("pstex_t"),
        id => lang.available() || driver_available(id),
    }
}

/// Export `doc` with fig2dev's driver for `lang`: `options` are fig2dev's
/// option letters and arguments, in order (general ones too); `dir` is
/// where relative picture names are found. The output and fig2dev's
/// messages.
pub fn export_driver(
    doc: &Document,
    lang: &str,
    options: &[(char, Option<String>)],
    dir: Option<&Path>,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let mut dev = rustfig_fig2dev::drivers::driver(lang).ok_or_else(|| format!("no fig2dev driver for {lang}"))?;
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
    let status = rustfig_fig2dev::convert(&mut ctx, dev.as_mut(), &objects)?;
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
    read_picture(&Ctx::default(), &path.to_string_lossy())
}

/// The EPS pictures of `doc` that RustFig can't draw, for embedding in EPS
/// output (by file name; relative names relative to `dir`).
pub fn eps_pictures(
    doc: &Document,
    dir: Option<&Path>,
) -> std::collections::BTreeMap<String, rustfig_render::export::EmbeddedEps> {
    let mut out = std::collections::BTreeMap::new();
    doc.for_each_leaf(&mut |o| {
        let rustfig_model::ObjectKind::Polyline(p) = &o.kind else {
            return;
        };
        let Some(pic) = p
            .picture
            .as_ref()
            .filter(|pic| pic.image.is_none() && !pic.file.is_empty())
        else {
            return;
        };
        if out.contains_key(&pic.file) {
            return;
        }
        let path = crate::import::resolve(&pic.file, dir);
        if let Ok(LoadedPicture::Eps { llx, lly, urx, ury, ps }) = load_picture(&path) {
            out.insert(
                pic.file.clone(),
                rustfig_render::export::EmbeddedEps {
                    title: pic.file.clone(),
                    llx,
                    lly,
                    urx,
                    ury,
                    ps,
                },
            );
        }
    });
    out
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
