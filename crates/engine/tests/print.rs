#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B5: the Print panel's commands. Nothing here reaches a printer: the
//! engine's spooler is a recorder.

use rustfig_engine::print::{PrintJob, RecordingSpooler};
use rustfig_engine::Engine;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

fn engine() -> (Engine, Arc<Mutex<Vec<PrintJob>>>) {
    let mut e = Engine::new();
    let rec = RecordingSpooler::default();
    let jobs = rec.jobs.clone();
    e.set_spooler(Box::new(rec));
    e.execute(
        "shape.rectangle",
        &json!({"x": 600, "y": 600, "width": 3000, "height": 1500}),
    )
    .unwrap();
    (e, jobs)
}

fn pdf_of(e: &mut Engine, p: Value) -> (usize, f64, f64, Vec<u8>) {
    let r = e.execute("print.pdf", &p).unwrap();
    let b64 = r["base64"].as_str().unwrap();
    let bytes = decode(b64);
    assert!(bytes.starts_with(b"%PDF"));
    (
        r["pages"].as_u64().unwrap() as usize,
        r["width"].as_f64().unwrap(),
        r["height"].as_f64().unwrap(),
        bytes,
    )
}

fn decode(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

#[test]
fn the_pdf_is_a_full_page_of_the_paper() {
    let (mut e, _) = engine();
    e.execute("doc.set_paper", &json!({"paper": "A4"})).unwrap();
    e.execute("doc.set_page", &json!({"orientation": "portrait"})).unwrap();
    let (pages, w, h, _) = pdf_of(&mut e, json!({}));
    assert_eq!(pages, 1);
    assert!((w - 595.0).abs() < 2.0 && (h - 842.0).abs() < 2.0, "{w} x {h}");
    e.execute("doc.set_page", &json!({"orientation": "landscape"})).unwrap();
    let (_, w, h, _) = pdf_of(&mut e, json!({}));
    assert!(w > h, "landscape: {w} x {h}");
    e.execute("doc.set_paper", &json!({"paper": "Letter"})).unwrap();
    let (_, w, _, _) = pdf_of(&mut e, json!({}));
    assert!((w - 792.0).abs() < 2.0, "{w}");
}

#[test]
fn magnification_makes_multiple_pages() {
    let (mut e, _) = engine();
    e.execute("doc.set_paper", &json!({"paper": "A4"})).unwrap();
    e.execute("doc.set_magnification", &json!({"magnification": 800}))
        .unwrap();
    let (single, ..) = pdf_of(&mut e, json!({}));
    assert_eq!(single, 1);
    e.execute("doc.set_page", &json!({"pages": "multiple"})).unwrap();
    let (multi, ..) = pdf_of(&mut e, json!({}));
    assert!(multi > 1, "{multi} pages");
    let (overlap, ..) = pdf_of(&mut e, json!({"settings": {"overlap": true}}));
    assert!(overlap >= multi);
}

#[test]
fn background_grid_and_layers_reach_fig2dev() {
    let (mut e, _) = engine();
    let (.., plain) = pdf_of(&mut e, json!({}));
    let (.., grid) = pdf_of(&mut e, json!({"settings": {"grid_minor": "1/4", "grid_major": "1"}}));
    assert_ne!(plain, grid, "the grid changes the page");
    let (.., bg) = pdf_of(&mut e, json!({"settings": {"background": 4}}));
    assert_ne!(plain, bg, "the background changes the page");
    e.execute(
        "shape.ellipse",
        &json!({"cx": 2000, "cy": 3000, "rx": 500, "ry": 300, "depth": 70}),
    )
    .unwrap();
    let (.., all) = pdf_of(&mut e, json!({}));
    e.execute("depth.set_visible", &json!({"depth": 70, "visible": false}))
        .unwrap();
    let (.., active) = pdf_of(&mut e, json!({"settings": {"all_layers": false}}));
    assert_ne!(all, active, "a hidden depth is left out");
    let (.., still_all) = pdf_of(&mut e, json!({}));
    assert_eq!(all, still_all, "all layers prints hidden depths too");
}

#[test]
fn print_goes_to_the_spooler() {
    let (mut e, jobs) = engine();
    let r = e.execute("print.run", &json!({})).unwrap();
    assert_eq!(r["printed"], 1);
    let jobs = jobs.lock().unwrap();
    assert_eq!(jobs.len(), 1);
    assert!(jobs[0].pdf.starts_with(b"%PDF"));
}

#[test]
fn a_cancelled_dialog_prints_nothing_and_keeps_the_batch() {
    let (mut e, _) = engine();
    let rec = RecordingSpooler {
        cancel: true,
        ..RecordingSpooler::default()
    };
    let jobs = rec.jobs.clone();
    e.set_spooler(Box::new(rec));
    let r = e.execute("print.run", &json!({})).unwrap();
    assert_eq!(r["printed"], 0);
    e.execute("print.batch_add", &json!({})).unwrap();
    let r = e.execute("print.batch_print", &json!({})).unwrap();
    assert_eq!(r["printed"], 0);
    assert_eq!(e.print_batch_len(), 1);
    assert!(jobs.lock().unwrap().is_empty());
}

#[test]
fn quick_print_uses_the_kept_settings() {
    let (mut e, jobs) = engine();
    e.execute("print.quick", &json!({})).unwrap();
    assert_eq!(jobs.lock().unwrap().len(), 1);
}

#[test]
fn old_printer_settings_are_ignored_in_the_settings_file() {
    let back: rustfig_engine::prefs::Prefs =
        toml::from_str("[rustfig.print]\noverlap = true\nprinter = \"hp\"\nparams = \"-n 2\"\nsystem_dialog = false\n")
            .unwrap();
    assert!(back.rustfig.print.overlap);
}

#[test]
fn settings_are_kept_and_reset() {
    let (mut e, _) = engine();
    e.execute(
        "print.settings",
        &json!({"settings": {"overlap": true, "grid_minor": "1/2"}}),
    )
    .unwrap();
    assert!(e.prefs_dirty);
    let r = e.execute("print.settings", &json!({})).unwrap();
    assert_eq!(r["settings"]["overlap"], true);
    assert_eq!(r["settings"]["grid_minor"], "1/2");
    assert_eq!(r["batch"], 0);
    assert_eq!(r["pages"], "single");
    // Through the settings file and back.
    let text = toml::to_string(&e.prefs).unwrap();
    let back: rustfig_engine::prefs::Prefs = toml::from_str(&text).unwrap();
    assert!(back.rustfig.print.overlap);
    let r = e.execute("print.settings", &json!({"reset": true})).unwrap();
    assert_eq!(r["settings"]["overlap"], false);
    assert!(e.execute("print.settings", &json!({"settings": {"bogus": 1}})).is_err());
    assert!(e
        .execute("print.settings", &json!({"settings": {"printer": "hp"}}))
        .is_err());
}

#[test]
fn batch_add_clear_and_print() {
    let (mut e, jobs) = engine();
    assert!(e.execute("print.batch_print", &json!({})).is_err());
    assert_eq!(e.execute("print.batch_add", &json!({})).unwrap()["batch"], 1);
    e.execute("doc.set_paper", &json!({"paper": "Letter"})).unwrap();
    assert_eq!(e.execute("print.batch_add", &json!({})).unwrap()["batch"], 2);
    e.execute("print.batch_clear", &json!({})).unwrap();
    assert_eq!(e.print_batch_len(), 0);
    e.execute("print.batch_add", &json!({})).unwrap();
    e.execute("print.batch_add", &json!({})).unwrap();
    // Print leaves the batch alone; as in xfig, Quick Print sends the batch and clears it.
    assert_eq!(e.execute("print.run", &json!({})).unwrap()["printed"], 1);
    assert_eq!(e.print_batch_len(), 2);
    let r = e.execute("print.quick", &json!({})).unwrap();
    assert_eq!(r["printed"], 2);
    assert_eq!(e.print_batch_len(), 0);
    let jobs = jobs.lock().unwrap();
    assert_eq!(jobs.len(), 2, "the batch is one job");
    assert_eq!(rustfig_engine::print::pdf_info(&jobs[1].pdf).unwrap().0, 2);
}

#[test]
fn the_batch_is_one_job_with_a_page_per_figure() {
    let (mut e, jobs) = engine();
    for _ in 0..3 {
        e.execute("print.batch_add", &json!({})).unwrap();
    }
    let r = e.execute("print.batch_print", &json!({})).unwrap();
    assert_eq!(r["printed"], 3);
    assert_eq!(e.print_batch_len(), 0);
    let jobs = jobs.lock().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].name, "3 figures");
    assert_eq!(rustfig_engine::print::pdf_info(&jobs[0].pdf).unwrap().0, 3);
    for i in 0..3 {
        assert!(rustfig_engine::print::render_page(&jobs[0].pdf, i, 200).is_ok());
    }
}

#[test]
fn a_failing_spooler_keeps_the_batch() {
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 900, "height": 900}))
        .unwrap();
    e.execute("print.batch_add", &json!({})).unwrap();
    // The default spooler refuses: scripts and tests never print.
    let err = e.execute("print.batch_print", &json!({})).unwrap_err();
    assert!(err.message.contains("no print system"), "{}", err.message);
    assert_eq!(e.print_batch_len(), 1);
    let err = e.execute("print.run", &json!({})).unwrap_err();
    assert!(err.message.contains("no print system"));
}

#[test]
fn an_empty_figure_does_not_print() {
    let mut e = Engine::new();
    let rec = RecordingSpooler::default();
    let jobs = rec.jobs.clone();
    e.set_spooler(Box::new(rec));
    assert!(e.execute("print.run", &json!({})).is_err());
    assert!(e.execute("print.quick", &json!({})).is_err());
    assert!(e.execute("print.batch_add", &json!({})).is_err());
    assert!(jobs.lock().unwrap().is_empty());
}

#[test]
fn hostile_parameters_are_refused() {
    let (mut e, jobs) = engine();
    for s in [
        json!({"printer": "a\nb"}),
        json!({"params": "x ".repeat(5_000)}),
        json!({"overlap": 5}),
        json!({"background": "red"}),
        json!({"grid_minor": 7}),
    ] {
        for c in ["print.run", "print.pdf", "print.batch_add", "print.settings"] {
            assert!(e.execute(c, &json!({"settings": s})).is_err(), "{c} {s}");
        }
    }
    assert!(e.execute("print.run", &json!({"settings": 3})).is_err());
    assert_eq!(e.print_batch_len(), 0);
    assert!(jobs.lock().unwrap().is_empty());
}

#[test]
fn the_preview_renders_the_page() {
    let (mut e, _) = engine();
    let (pages, _, _, pdf) = pdf_of(&mut e, json!({}));
    let img = rustfig_engine::print::render_page(&pdf, 0, 300).unwrap();
    assert!(img.width.max(img.height) <= 300 && img.width > 50);
    assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
    assert!(
        img.rgba.as_chunks::<4>().0.iter().any(|p| p[0] < 128),
        "the rectangle's line is drawn"
    );
    assert!(img.rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    assert!(rustfig_engine::print::render_page(&pdf, pages, 300).is_err());
    assert!(rustfig_engine::print::render_page(b"not a pdf", 0, 300).is_err());
}
