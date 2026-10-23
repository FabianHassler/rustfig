//! The Print panel's commands (B5): its values, the PDF, printing, quick
//! print and the batch. Magnification, Fit to Page, paper, orientation,
//! justification and pages are the figure's own (`doc.set_magnification`,
//! `export.fit_page`, `doc.set_paper`, `doc.set_page`).

use super::{always, CmdResult, Command};
use crate::export_panel::figure_size;
use crate::print::{self, BatchItem, PrintSettings};
use crate::{Engine, EngineError};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "print.settings",
            label: "Print panel settings",
            menu: "",
            shortcut: "",
            params: "{\"settings\"?: {overlap, background (colour number, -1 none), grid_minor, grid_major (\"None\" or a spacing like \"1/4\"), all_layers (false: only the active layers), bound_active_layers, printer (\"\" = the default printer), params (options for lp, %f is the file name), system_dialog (true: the system's print dialog; false: straight to the printer)}, \"reset\"?: bool}  — without settings returns the ones kept; settings are merged into them and kept in the settings file across sessions (not in the drawing); reset goes back to xfig's defaults. Magnification, paper size, orientation, justification and single/multiple pages are the figure's own: doc.set_magnification, export.fit_page, doc.set_paper, doc.set_page. Returns {settings, magnification, paper, orientation, justify, pages, figure_width, figure_height, unit, batch}",
            mutates: false,
            enabled: always,
            run: settings,
        },
        Command {
            id: "print.pdf",
            label: "Print preview PDF",
            menu: "",
            shortcut: "",
            params: "{\"settings\"?: {…as print.settings, used without keeping them}}  — the PDF that Print would hand to the printer: {base64, pages, width, height (the page in points), warnings}",
            mutates: false,
            enabled: super::has_objects,
            run: pdf,
        },
        Command {
            id: "print.run",
            label: "Print",
            menu: "",
            shortcut: "",
            params: "{\"settings\"?: {…as print.settings, used without keeping them}}  — print the figure (the PDF to the system's print dialog, or to the printer named in the settings); the batch is left alone (print.batch_print sends it). Returns {message, printed (figures sent)}",
            mutates: false,
            enabled: always,
            run,
        },
        Command {
            id: "print.quick",
            label: "Print with current settings",
            menu: "",
            shortcut: "",
            params: "{}  — xfig's Shift-Meta-P: print with the kept settings, without the panel; as xfig, with figures waiting in the batch it sends the batch instead and clears it. Returns {message, printed}",
            mutates: false,
            enabled: always,
            run: quick,
        },
        Command {
            id: "print.batch_add",
            label: "Add figure to print batch",
            menu: "",
            shortcut: "",
            params: "{\"settings\"?: {…as print.settings, used without keeping them}}  — make the figure's PDF and keep it in the batch (xfig's \"Print FIGURE to Batch\"); returns {batch}",
            mutates: false,
            enabled: super::has_objects,
            run: batch_add,
        },
        Command {
            id: "print.batch_clear",
            label: "Clear print batch",
            menu: "",
            shortcut: "",
            params: "{}  — forget the figures waiting in the batch",
            mutates: false,
            enabled: |e| !e.print_batch.is_empty(),
            run: batch_clear,
        },
        Command {
            id: "print.batch_print",
            label: "Print the batch",
            menu: "",
            shortcut: "",
            params: "{\"settings\"?: {…as print.settings, used without keeping them}}  — send the batch's figures to the printer, one after the other, and clear it; returns {message, printed}",
            mutates: false,
            enabled: |e| !e.print_batch.is_empty(),
            run: batch_print,
        },
    ]
}

/// The kept settings with the command's `settings` merged in (not kept).
fn effective(e: &Engine, p: &Value) -> Result<PrintSettings, EngineError> {
    match p.get("settings").filter(|v| !v.is_null()) {
        Some(v) => print::merge(print::settings_of(e), v),
        None => Ok(print::settings_of(e)),
    }
}

fn settings(e: &mut Engine, p: &Value) -> CmdResult {
    if super::opt_bool(p, "reset")?.unwrap_or(false) {
        print::store(e, PrintSettings::default());
    }
    if let Some(v) = p.get("settings").filter(|v| !v.is_null()) {
        let merged = print::merge(print::settings_of(e), v)?;
        print::store(e, merged);
    }
    let d = &e.doc().settings;
    let (width, height, unit) = figure_size(&e.doc_bounds(), d.units, d.magnification);
    Ok(json!({
        "settings": serde_json::to_value(print::settings_of(e)).map_err(|err| EngineError::internal(err.to_string()))?,
        "magnification": d.magnification,
        "paper": d.paper.name(),
        "orientation": if d.orientation == rustfig_model::Orientation::Landscape { "landscape" } else { "portrait" },
        "justify": if d.center { "center" } else { "flush_left" },
        "pages": if d.multiple_pages { "multiple" } else { "single" },
        "figure_width": width,
        "figure_height": height,
        "unit": unit,
        "batch": e.print_batch.len(),
    }))
}

fn pdf(e: &mut Engine, p: &Value) -> CmdResult {
    let s = effective(e, p)?;
    let (bytes, warnings) = print::make_pdf(e, &s)?;
    let (pages, width, height) = print::pdf_info(&bytes).unwrap_or((0, 0.0, 0.0));
    Ok(json!({
        "base64": super::file::base64(&bytes),
        "pages": pages,
        "width": width,
        "height": height,
        "warnings": warnings,
    }))
}

/// Send one PDF to the spooler; the message to show.
fn submit(e: &mut Engine, s: &PrintSettings, name: String, pdf: Vec<u8>) -> Result<String, EngineError> {
    let job = print::job_for(e, s, name, pdf);
    let sent = e.spooler.submit(&job).map_err(EngineError::io)?;
    Ok(if s.system_dialog {
        sent
    } else if s.printer.is_empty() {
        format!("Printing on default printer with {} ... done", print::mode_text(e))
    } else {
        format!("Printing on \"{}\" with {} ... done", s.printer, print::mode_text(e))
    })
}

fn print_now(e: &mut Engine, s: &PrintSettings, batch_first: bool) -> CmdResult {
    // xfig's Print and Quick Print send the batch when there is one.
    if batch_first && !e.print_batch.is_empty() {
        return print_batch_with(e, s);
    }
    let (bytes, warnings) = print::make_pdf(e, s)?;
    let name = print::job_name(e);
    let message = submit(e, s, name, bytes)?;
    e.set_status(message.clone());
    Ok(json!({"message": message, "printed": 1, "warnings": warnings}))
}

fn run(e: &mut Engine, p: &Value) -> CmdResult {
    let s = effective(e, p)?;
    print_now(e, &s, false)
}

fn quick(e: &mut Engine, _: &Value) -> CmdResult {
    let s = print::settings_of(e);
    print_now(e, &s, true)
}

fn batch_add(e: &mut Engine, p: &Value) -> CmdResult {
    let s = effective(e, p)?;
    if e.print_batch.len() >= print::MAX_BATCH_FIGURES {
        return Err(EngineError::bad_params(format!(
            "the batch holds {} figures already; print or clear it",
            print::MAX_BATCH_FIGURES
        )));
    }
    let (bytes, warnings) = print::make_pdf(e, &s)?;
    let held: usize = e.print_batch.iter().map(|b| b.pdf.len()).sum();
    if held.saturating_add(bytes.len()) > print::MAX_BATCH_BYTES {
        return Err(EngineError::bad_params("the batch is full; print or clear it"));
    }
    let name = print::job_name(e);
    e.print_batch.push(BatchItem { name, pdf: bytes });
    let n = e.print_batch.len();
    e.set_status(format!(
        "Appending to the print batch ({}) ... done",
        print::mode_text(e)
    ));
    Ok(json!({"batch": n, "warnings": warnings}))
}

fn batch_clear(e: &mut Engine, _: &Value) -> CmdResult {
    e.print_batch.clear();
    e.set_status("Print batch cleared");
    Ok(json!({"batch": 0}))
}

fn batch_print(e: &mut Engine, p: &Value) -> CmdResult {
    let s = effective(e, p)?;
    if e.print_batch.is_empty() {
        return Err(EngineError::bad_params("the print batch is empty"));
    }
    print_batch_with(e, &s)
}

/// Send every figure of the batch in order and clear it; if one fails, it
/// and those after it stay.
fn print_batch_with(e: &mut Engine, s: &PrintSettings) -> CmdResult {
    let mut printed = 0usize;
    let mut message = String::new();
    while !e.print_batch.is_empty() {
        let item = e.print_batch.remove(0);
        let job_name = item.name.clone();
        match submit(e, s, item.name.clone(), item.pdf.clone()) {
            Ok(m) => {
                message = m;
                printed += 1;
            }
            Err(err) => {
                e.print_batch.insert(0, item);
                return Err(EngineError::io(format!(
                    "{job_name}: {} ({printed} sent, {} waiting)",
                    err.message,
                    e.print_batch.len()
                )));
            }
        }
    }
    let message = format!(
        "Printed {printed} figure{} from the batch: {message}",
        if printed == 1 { "" } else { "s" }
    );
    e.set_status(message.clone());
    Ok(json!({"message": message, "printed": printed}))
}
