//! The Export panel's commands (W10-4, W10-11): its values per language,
//! fit to page, and the figure's page settings.

use super::{always, has_objects, opt_bool, opt_str, string, CmdResult, Command};
use crate::export_panel::{figure_size, fit_page, merge, rows, settings_for, store, PanelSettings};
use crate::fig2dev::languages::language;
use crate::{Engine, EngineError};
use rustfig_model::Orientation;
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "export.settings",
            label: "Export panel settings",
            menu: "",
            shortcut: "",
            params: "{\"format\": language id, \"settings\"?: {border (points; pixels for bitmaps), background (colour number, -1 none), grid_minor, grid_major (\"None\" or a spacing like \"1/4\"), preview (0 none, 1 ASCII, 2 colour TIFF, 3 monochrome TIFF), pdf_page_mode, overlap, x_offset, y_offset, x_unit, y_unit (0 in, 1 cm, 2 fig units), transparent (colour number, -2 none, -3 background), dpi, smoothing (1, 2, 4), quality}, \"reset\"?: bool}  — without settings returns the ones kept for the language; settings are merged into them and kept in the settings file across sessions (not in the drawing); reset: true goes back to xfig's defaults. Returns {format, settings, rows} where rows says which panel rows the language shows. file.export takes \"panel\": true (these settings) or \"panel\": {…} (these, merged into them) and turns them into fig2dev's options",
            mutates: false,
            enabled: always,
            run: settings,
        },
        Command {
            id: "export.fit_page",
            label: "Fit to page",
            menu: "",
            shortcut: "",
            params: "{}  — set the figure's magnification so that it fits the paper (a half inch of margin all round, as xfig's \"Fit to Page\"); returns {magnification, width, height, unit} (the figure's size then)",
            mutates: true,
            enabled: has_objects,
            run: fit,
        },
        Command {
            id: "export.figure_size",
            label: "Figure size",
            menu: "",
            shortcut: "",
            params: "{\"magnification\"?: percent (default: the figure's)}  — the figure's size in the current unit at that magnification: {width, height, unit}",
            mutates: false,
            enabled: always,
            run: size,
        },
        Command {
            id: "doc.set_page",
            label: "Set page options",
            menu: "",
            shortcut: "",
            params: "{\"orientation\"?: landscape|portrait, \"justify\"?: center|flush_left, \"pages\"?: single|multiple}  — the figure's orientation, justification and single/multiple pages (stored in the .fig file; the paper size is doc.set_paper)",
            mutates: true,
            enabled: always,
            run: page,
        },
    ]
}

fn lang_id(p: &Value) -> Result<&'static str, EngineError> {
    let f = string(p, "format")?;
    language(&f)
        .map(|l| l.id)
        .ok_or_else(|| EngineError::bad_params(format!("unknown export language {f:?} (see file.export_formats)")))
}

fn settings(e: &mut Engine, p: &Value) -> CmdResult {
    let id = lang_id(p)?;
    if opt_bool(p, "reset")?.unwrap_or(false) {
        store(e, id, PanelSettings::xfig());
    }
    if let Some(v) = p.get("settings").filter(|v| !v.is_null()) {
        let merged = merge(settings_for(e, id), v)?;
        store(e, id, merged);
    }
    Ok(json!({
        "format": id,
        "settings": serde_json::to_value(settings_for(e, id)).map_err(|err| EngineError::bad_params(err.to_string()))?,
        "rows": serde_json::to_value(rows(id)).map_err(|err| EngineError::bad_params(err.to_string()))?,
    }))
}

fn fit(e: &mut Engine, _: &Value) -> CmdResult {
    let bounds = e.doc_bounds();
    let s = &e.doc().settings;
    let m = fit_page(&bounds, s.paper, s.orientation == Orientation::Landscape, s.units)
        .ok_or_else(|| EngineError::bad_params("the figure is empty"))?;
    let units = s.units;
    e.doc_mut().settings.magnification = m;
    let (width, height, unit) = figure_size(&bounds, units, m);
    Ok(json!({"magnification": m, "width": width, "height": height, "unit": unit}))
}

fn size(e: &mut Engine, p: &Value) -> CmdResult {
    let m = super::opt_num(p, "magnification")?.unwrap_or(e.doc().settings.magnification);
    if !(m.is_finite() && m > 0.0) {
        return Err(EngineError::bad_params("magnification must be a positive percentage"));
    }
    let (width, height, unit) = figure_size(&e.doc_bounds(), e.doc().settings.units, m);
    Ok(json!({"width": width, "height": height, "unit": unit}))
}

fn page(e: &mut Engine, p: &Value) -> CmdResult {
    let o = opt_str(p, "orientation")?;
    let j = opt_str(p, "justify")?;
    let n = opt_str(p, "pages")?;
    let orientation = match o {
        None => None,
        Some(s) if s.eq_ignore_ascii_case("landscape") => Some(Orientation::Landscape),
        Some(s) if s.eq_ignore_ascii_case("portrait") => Some(Orientation::Portrait),
        Some(s) => return Err(EngineError::bad_params(format!("unknown orientation {s:?}"))),
    };
    let center = match j {
        None => None,
        Some(s) if s.eq_ignore_ascii_case("center") => Some(true),
        Some(s) if s.eq_ignore_ascii_case("flush_left") => Some(false),
        Some(s) => return Err(EngineError::bad_params(format!("unknown justification {s:?}"))),
    };
    let multiple = match n {
        None => None,
        Some(s) if s.eq_ignore_ascii_case("single") => Some(false),
        Some(s) if s.eq_ignore_ascii_case("multiple") => Some(true),
        Some(s) => return Err(EngineError::bad_params(format!("unknown pages {s:?}"))),
    };
    let s = &mut e.doc_mut().settings;
    if let Some(o) = orientation {
        s.orientation = o;
    }
    if let Some(c) = center {
        s.center = c;
    }
    if let Some(m) = multiple {
        s.multiple_pages = m;
    }
    Ok(json!({}))
}
