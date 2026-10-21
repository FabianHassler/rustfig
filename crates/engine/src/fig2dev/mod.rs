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

pub use rustfig_format::bridge::to_model;

pub use fig2dev::util::readpics::LoadedPicture;
pub use fig2dev::{languages, options};

use fig2dev::object::Compound;
use fig2dev::{read, Ctx, Driver};
use languages::Language;
use rustfig_model::Document;
use std::path::Path;

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
///
/// The drivers get the objects straight from the document
/// ([`rustfig_format::bridge::export_objects`]), the same as through
/// [`export_via_text`] (plan D7); where the Fig file would make fig2dev's
/// reader say something or read it differently, the text path runs.
pub fn export(
    doc: &Document,
    lang: &str,
    options: &[(char, Option<String>)],
    dir: Option<&Path>,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let (mut dev, mut ctx) = setup(lang, options, dir)?;
    match rustfig_format::bridge::export_objects(doc, &mut ctx) {
        Some(objects) => finish(lang, dev.as_mut(), &mut ctx, &objects),
        None => export_via_text(doc, lang, options, dir),
    }
}

/// [`export`] the way xfig does it: the document written as Fig 3.2 text
/// and read by fig2dev's reader. The reference for [`export`].
pub fn export_via_text(
    doc: &Document,
    lang: &str,
    options: &[(char, Option<String>)],
    dir: Option<&Path>,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let (mut dev, mut ctx) = setup(lang, options, dir)?;
    let fig = rustfig_format::write_fig(doc);
    let objects = read::readfp_fig(&mut ctx, fig.as_bytes()).map_err(|_| ctx.messages.join("\n"))?;
    finish(lang, dev.as_mut(), &mut ctx, &objects)
}

/// The driver and the context with fig2dev's options applied.
fn setup(lang: &str, options: &[(char, Option<String>)], dir: Option<&Path>) -> Result<(Box<dyn Driver>, Ctx), String> {
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
    Ok((dev, ctx))
}

fn finish(
    lang: &str,
    dev: &mut dyn Driver,
    ctx: &mut Ctx,
    objects: &Compound,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let status = fig2dev::convert(ctx, dev, objects)?;
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
