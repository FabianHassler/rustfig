//! fig2dev's reader and drivers in the engine: old Fig files open (G1,
//! S3), the ported drivers export (B9), pstex/pdftex write their two
//! parts (B2), EPS pictures are embedded in EPS exports.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::Engine;
use serde_json::{json, Value};
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fig2dev-bridge-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A Fig 2.1 file (80 ppi): a box, a dashed red line with an arrow, an
/// approximated spline and a PostScript text.
const FIG21: &str = "#FIG 2.1\n80 2\n\
    2 2 0 1 -1 0 0 0 0.000 7 0 0\n\t 10 10 90 10 90 50 10 50 10 10 9999 9999\n\
    2 1 1 2 4 0 0 0 4.000 -1 1 0\n\t0 0 1.000 4.000 8.000\n\t 10 70 50 70 9999 9999\n\
    3 0 0 1 -1 0 0 0 0.000 0 0\n\t 10 100 50 120 90 100 9999 9999\n\
    4 0 -1 0 0 0 12 0.00000 4 9 30 10 150 Hello\x01\n";

/// A Fig 1.3 file: a header of numbers, an open spline.
const FIG13: &str = "80 2 9 9\n3 10 0 1 1 0 0 0 0 0 0\n10 10 40 0 9999 0\n";

fn types(e: &Engine) -> Vec<&'static str> {
    e.doc().objects.iter().map(|o| o.type_name()).collect()
}

#[test]
fn fig_2_1_and_1_3_files_open() {
    let dir = temp_dir("old");
    let p21 = dir.join("old21.fig");
    std::fs::write(&p21, FIG21).unwrap();
    let mut e = Engine::new();
    let r = e.execute("file.open", &json!({"path": p21})).unwrap();
    assert_eq!(r["objects"], 4);
    assert_eq!(types(&e), ["box", "polyline", "spline", "text"]);
    let x = e.doc_bounds().max.x;
    assert!((1350.0..1370.0).contains(&x), "80 ppi scaled to 1200: {x}");
    // Saved as Fig 3.2, it reads back with the reader.
    let saved = dir.join("saved.fig");
    e.execute("file.save", &json!({"path": saved})).unwrap();
    let text = std::fs::read_to_string(&saved).unwrap();
    assert!(text.starts_with("#FIG 3.2"), "{text}");
    let mut e2 = Engine::new();
    e2.execute("file.open", &json!({"path": saved})).unwrap();
    assert_eq!(types(&e2), types(&e));

    let p13 = dir.join("old13.fig");
    std::fs::write(&p13, FIG13).unwrap();
    let r = e.execute("file.open", &json!({"path": p13})).unwrap();
    assert_eq!(r["objects"], 1);
    assert_eq!(types(&e), ["spline"]);

    // And from data, with the reader's messages as warnings: like xfig, it
    // deletes a box of fewer than 5 points.
    let open_box = "#FIG 2.1\n80 2\n2 2 0 1 -1 0 0 0 0.000 7 0 0\n\t 10 10 90 10 90 50 10 50 9999 9999\n";
    let r = e.execute("file.open_data", &json!({"data": open_box})).unwrap();
    assert_eq!(r["objects"], 0);
    let w = r["warnings"].as_array().unwrap();
    assert!(
        w.iter().any(|m| m.as_str().unwrap().contains("Deleting zero-size box")),
        "{r}"
    );
    e.execute("file.merge_data", &json!({"data": FIG21})).unwrap();
    assert_eq!(e.doc().objects.len(), 4);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bad_old_files_are_errors_and_change_nothing() {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": FIG21})).unwrap();
    for bad in [
        "#FIG 2.1\n",
        "#FIG 2.1\n80 2\n2 9 9\n",
        "12 x\n",
        "#FIG 1.4\n80 2\n7 1 2 3\n",
    ] {
        let before = e.doc().clone();
        assert!(e.execute("file.open_data", &json!({"data": bad})).is_err(), "{bad:?}");
        assert!(e.execute("file.merge_data", &json!({"data": bad})).is_err(), "{bad:?}");
        assert_eq!(*e.doc(), before);
    }
}

/// A box, a PostScript text and a special (LaTeX) text.
const DRAWING: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
    2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 2400 0 2400 1200 0 1200 0 0\n\
    4 0 0 50 -1 0 12 0.0000 4 135 450 300 600 plain\\001\n\
    4 0 0 50 -1 0 12 0.0000 2 135 450 300 900 $\\\\alpha^2$\\001\n";

fn drawing() -> Engine {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": DRAWING})).unwrap();
    e
}

fn special_count(e: &Engine) -> usize {
    let mut n = 0;
    e.doc().for_each_leaf(&mut |o| {
        if let rustfig_model::ObjectKind::Text(t) = &o.kind {
            n += usize::from(t.is_special());
        }
    });
    n
}

#[test]
fn ported_drivers_export_and_are_listed() {
    let mut e = drawing();
    let v = e.execute("file.export_formats", &json!({})).unwrap();
    let latex = v["formats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "latex")
        .unwrap()
        .clone();
    assert_eq!(latex["available"], true, "{latex}");
    assert_eq!(latex["status"], "available");
    let letters: Vec<&str> = latex["options"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["available"] == true)
        .map(|o| o["letter"].as_str().unwrap())
        .collect();
    assert!(letters.contains(&"v") && letters.contains(&"m"), "{letters:?}");

    let r = e.execute("file.export_data", &json!({"format": "latex"})).unwrap();
    let text = r["data"].as_str().unwrap();
    assert!(
        text.contains("\\begin{picture}") && text.contains("\\end{picture}"),
        "{text}"
    );
    // Options by letter reach the driver; general ones by key too.
    let verbose = e
        .execute("file.export_data", &json!({"format": "latex", "options": {"v": true}}))
        .unwrap();
    assert_ne!(verbose["data"], r["data"]);
    let big = e
        .execute(
            "file.export_data",
            &json!({"format": "latex", "options": {"magnification": 2}}),
        )
        .unwrap();
    assert_ne!(big["data"], r["data"]);
    let pct = e
        .execute("file.export_data", &json!({"format": "latex", "magnification": 200}))
        .unwrap();
    assert_eq!(pct["data"], big["data"]);
    // Unknown letters and keys are refused.
    for bad in [json!({"q": true}), json!({"nonsense": 1}), json!({"v": [1]})] {
        let err = e
            .execute("file.export_data", &json!({"format": "latex", "options": bad}))
            .unwrap_err();
        assert!(err.message.contains("option"), "{}", err.message);
    }

    // Every language the crate writes exports through file.export_data.
    for f in v["formats"].as_array().unwrap() {
        let id = f["id"].as_str().unwrap();
        if f["available"] == true {
            // A driver may refuse a figure (shape wants comments), as fig2dev does.
            let r = e.execute("file.export_data", &json!({"format": id}));
            assert!(
                r.as_ref().map_or_else(|err| err.code == "format", |_| true),
                "{id}: {r:?}"
            );
        }
    }
}

#[test]
fn pstex_writes_graphics_without_special_text_and_the_latex_part() {
    let mut e = drawing();
    assert_eq!(special_count(&e), 1);
    let dir = temp_dir("pstex");
    let path = dir.join("fig.pstex").to_string_lossy().into_owned();
    let r = e
        .execute("file.export", &json!({"path": path, "format": "pstex"}))
        .unwrap();
    let paths: Vec<&str> = r["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(paths, [path.clone(), format!("{path}_t")]);
    let eps = std::fs::read_to_string(&path).unwrap();
    assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0"), "{eps}");
    assert!(eps.contains("(plain)") && !eps.contains("alpha"), "{eps}");
    let tex = std::fs::read_to_string(format!("{path}_t")).unwrap();
    assert!(tex.contains("\\includegraphics{fig.pstex}"), "{tex}");
    assert!(tex.contains("$\\alpha^2$") && !tex.contains("plain"), "{tex}");

    // The EPS and the overlay have the same size.
    let bb: Vec<f64> = eps
        .lines()
        .find(|l| l.starts_with("%%BoundingBox:"))
        .unwrap()
        .split_whitespace()
        .skip(1)
        .map(|n| n.parse().unwrap())
        .collect();
    assert!(bb.len() == 4 && bb[2] > 0.0, "{eps}");
    assert!(
        tex.lines().filter(|l| l.starts_with("\\begin{picture}(")).count() == 2,
        "{tex}"
    );

    // pdftex: NAME.pdf and NAME.pdf_t.
    let pdf = dir.join("fig.pdf").to_string_lossy().into_owned();
    e.execute("file.export", &json!({"path": pdf, "format": "pdftex"}))
        .unwrap();
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    let tex = std::fs::read_to_string(format!("{pdf}_t")).unwrap();
    assert!(tex.contains("\\includegraphics{fig.pdf}"), "{tex}");

    // pspdftex: NAME_pspdftex.eps, .pdf and the LaTeX part at PATH.
    let both = dir.join("fig.pspdftex").to_string_lossy().into_owned();
    let r = e
        .execute(
            "file.export",
            &json!({"path": both, "format": "pspdftex", "options": {"border": 5}}),
        )
        .unwrap();
    assert_eq!(r["paths"].as_array().unwrap().len(), 3, "{r}");
    assert!(dir.join("fig_pspdftex.eps").is_file() && dir.join("fig_pspdftex.pdf").is_file());
    let tex = std::fs::read_to_string(&both).unwrap();
    assert!(tex.contains("\\includegraphics{fig_pspdftex}"), "{tex}");

    // export_data returns every part.
    let r = e.execute("file.export_data", &json!({"format": "pstex"})).unwrap();
    let files = r["files"].as_array().unwrap();
    let ext: Vec<&str> = files.iter().map(|f| f["extension"].as_str().unwrap()).collect();
    assert_eq!(ext, ["pstex", "pstex_t"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn eps_pictures_are_embedded_in_eps() {
    let dir = temp_dir("embed");
    std::fs::write(
        dir.join("inner.eps"),
        "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 10 20 110 70\nnewpath 10 20 moveto 110 70 lineto stroke\n%%EOF\n",
    )
    .unwrap();
    let fig = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
        2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 inner.eps\n\t 0 0 2400 0 2400 1200 0 1200 0 0\n";
    let path = dir.join("withpic.fig");
    std::fs::write(&path, fig).unwrap();
    let mut e = Engine::new();
    let r = e.execute("file.open", &json!({"path": path})).unwrap();
    let w: Vec<Value> = r["warnings"].as_array().unwrap().clone();
    assert!(w.iter().any(|m| m.as_str().unwrap().contains("EPS picture")), "{r}");
    let eps = e.execute("file.export_data", &json!({"format": "eps"})).unwrap();
    let eps = eps["data"].as_str().unwrap();
    assert!(eps.contains("% Begin Imported EPS File: inner.eps"), "{eps}");
    assert!(eps.contains("110 70 lineto stroke"), "{eps}");
    assert!(eps.contains("/showpage {} def"), "{eps}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn compressed_and_xbm_pictures_load() {
    let dir = temp_dir("pics");
    std::fs::write(
        dir.join("dot.xbm"),
        "#define dot_width 8\n#define dot_height 2\nstatic char dot_bits[] = {\n 0x01, 0x80 };\n",
    )
    .unwrap();
    let fig = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
        2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 dot.xbm\n\t 0 0 2400 0 2400 1200 0 1200 0 0\n";
    let path = dir.join("xbm.fig");
    std::fs::write(&path, fig).unwrap();
    let mut e = Engine::new();
    let r = e.execute("file.open", &json!({"path": path})).unwrap();
    assert_eq!(r["warnings"].as_array().unwrap().len(), 0, "{r}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// D2: the frame the engine fits and reports is the frame fig2dev's EPS
/// driver exports (points: 72/1200 of a Fig unit, rounded outward).
#[test]
fn doc_bounds_are_the_eps_frame() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().is_none_or(|x| x != "fig") {
            continue;
        }
        let mut e = Engine::new();
        e.execute("file.open", &json!({"path": p})).unwrap();
        let r = e
            .execute("file.export_data", &json!({"format": "eps", "options": {}}))
            .unwrap();
        let eps = r["data"].as_str().unwrap();
        let line = eps.lines().find(|l| l.starts_with("%%BoundingBox:")).unwrap();
        let v: Vec<f64> = line.split_whitespace().skip(1).map(|n| n.parse().unwrap()).collect();
        let b = e.doc_bounds();
        // Metric drawings have 450 units per cm, 1143 per inch.
        let k = if e.doc().settings.units == rustfig_model::Units::Metric {
            72.0 / 1143.0
        } else {
            0.06
        };
        let (w, h) = ((b.max.x - b.min.x) * k, (b.max.y - b.min.y) * k);
        // fig2dev rounds the corners outward; the extent differs by under 2 points.
        assert!(
            (v[2] - v[0] - w).abs() < 2.0,
            "{}: width {} vs {w}",
            p.display(),
            v[2] - v[0]
        );
        assert!(
            (v[3] - v[1] - h).abs() < 2.0,
            "{}: height {} vs {h}",
            p.display(),
            v[3] - v[1]
        );
        n += 1;
    }
    assert!(n >= 10);
}

/// D2: arrowheads count, and the selection box is fig2dev's.
#[test]
fn arrowheads_count_in_the_bounds() {
    let mut e = Engine::new();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [1200, 0]]}))
        .unwrap();
    let plain = e.doc_bounds();
    e.execute("edit.select_all", &json!({})).unwrap();
    e.execute("object.set_attributes", &json!({"forward_arrow": true}))
        .unwrap();
    let arrowed = e.doc_bounds();
    assert!(
        arrowed.max.x > plain.max.x || arrowed.max.y > plain.max.y,
        "{plain:?} {arrowed:?}"
    );
    assert_eq!(e.selection_bbox(), arrowed);
}

// ---- D7: export straight from the objects equals the text path ----

mod identity {
    use rustfig_engine::fig2dev::{driver_available, export, export_via_text, languages::LANGUAGES};
    use rustfig_model::Document;
    use serde_json::Value;
    use std::path::Path;

    type Opts = Vec<(char, Option<String>)>;
    type Exported = Result<(Vec<u8>, Vec<String>), String>;

    fn languages() -> Vec<&'static str> {
        let mut v: Vec<&str> = LANGUAGES
            .iter()
            .map(|l| l.id)
            .filter(|id| driver_available(id))
            .collect();
        for extra in ["pstex_t", "pdftex_t"] {
            if driver_available(extra) && !v.contains(&extra) {
                v.push(extra);
            }
        }
        v
    }

    fn opt(c: char, a: &str) -> (char, Option<String>) {
        (c, Some(a.to_string()))
    }

    fn option_sets() -> Vec<Opts> {
        vec![
            vec![],
            vec![opt('m', "0.5")],
            vec![opt('s', "14")],
            vec![opt('b', "10")],
            vec![opt('D', "-50")],
            vec![opt('Z', "3")],
            vec![('K', None)],
            vec![('c', None), opt('l', "portrait"), opt('p', "a4")],
            vec![opt('m', "1.5"), opt('s', "10"), opt('b', "5")],
        ]
    }

    fn run_both(doc: &Document, lang: &str, o: &Opts) -> (Exported, Exported) {
        (export(doc, lang, o, None), export_via_text(doc, lang, o, None))
    }

    /// Check one export; the clock in the headers may tick between the two.
    fn check(doc: &Document, lang: &str, o: &Opts, what: &str) {
        let (a, b) = run_both(doc, lang, o);
        if a != b {
            let (a, b) = run_both(doc, lang, o);
            if a != b {
                let show = |r: &Exported| match r {
                    Ok((out, m)) => format!("ok {} bytes, messages {m:?}", out.len()),
                    Err(e) => format!("error {e}"),
                };
                let at = match (&a, &b) {
                    (Ok((x, _)), Ok((y, _))) => x.iter().zip(y).position(|(p, q)| p != q),
                    _ => None,
                };
                let around = |r: &Exported| match (r, at) {
                    (Ok((out, _)), Some(i)) => String::from_utf8_lossy(
                        out.get(i.saturating_sub(80)..(i + 80).min(out.len()))
                            .unwrap_or_default(),
                    )
                    .into_owned(),
                    _ => String::new(),
                };
                if let Some(l) = rustfig_format::write_fig(doc)
                    .lines()
                    .find(|l| l.contains(['\0', '\r']))
                {
                    eprintln!("a line with NUL or CR: {l:?}");
                }
                panic!(
                    "{what} -L {lang} {o:?}: objects {} / text {} (first difference at {at:?})\nobjects: {}\ntext: {}",
                    show(&a),
                    show(&b),
                    around(&a),
                    around(&b)
                );
            }
        }
    }

    fn all_languages(doc: &Document, what: &str, sets: &[Opts]) -> usize {
        let mut n = 0;
        for lang in languages() {
            for o in sets {
                check(doc, lang, o, what);
                n += 1;
            }
        }
        n
    }

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn set_clock() {
        std::env::set_var("SOURCE_DATE_EPOCH", "1700000000");
    }

    fn load(p: &Path) -> Option<Document> {
        rustfig_format::read_any(&std::fs::read(p).ok()?).ok()
    }

    fn objects_used(doc: &Document) -> bool {
        rustfig_format::bridge::export_objects(doc, &mut fig2dev::Ctx::default()).is_some()
    }

    #[test]
    fn fixtures_export_identically_in_every_language_and_option() {
        set_clock();
        let dir = root().join("crates/engine/tests/fixtures");
        let mut n = 0;
        let mut used = 0;
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "fig") {
                let doc = load(&p).unwrap();
                used += usize::from(objects_used(&doc));
                n += all_languages(&doc, &p.display().to_string(), &option_sets());
            }
        }
        eprintln!("fixtures: {n} exports identical, {used} figures through objects");
        assert!(used >= 10);
    }

    /// The 300-figure sample of the corpus checks (local, needs `old/`).
    #[test]
    fn corpus_sample_exports_identically() {
        set_clock();
        let list = root().join("plan/corpus-sample.txt");
        let (Ok(text), true) = (std::fs::read_to_string(list), root().join("old").is_dir()) else {
            eprintln!("skipped: no plan/corpus-sample.txt or old/");
            return;
        };
        let sets = option_sets();
        let langs = languages();
        let (mut n, mut figures, mut used) = (0, 0, 0);
        for (k, line) in text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .enumerate()
        {
            let Some(doc) = load(&root().join(line)) else { continue };
            figures += 1;
            used += usize::from(objects_used(&doc));
            // Every language once, with a rotating set of options.
            for (j, lang) in langs.iter().enumerate() {
                check(&doc, lang, &sets[(k + j) % sets.len()], line);
                n += 1;
            }
            // And the main ones with all option sets.
            for lang in ["eps", "svg", "pdf"] {
                for o in &sets {
                    check(&doc, lang, o, line);
                    n += 1;
                }
            }
        }
        eprintln!("corpus sample: {figures} figures ({used} through objects), {n} exports identical");
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 33
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n.max(1) as u64) as usize
        }
    }

    /// JSON pointers of the numbers, strings and booleans in `v`.
    fn leaves(v: &Value, at: &mut String, out: &mut Vec<String>) {
        let len = at.len();
        match v {
            Value::Number(_) | Value::String(_) | Value::Bool(_) => out.push(at.clone()),
            Value::Array(a) => {
                for (i, x) in a.iter().enumerate() {
                    at.push_str(&format!("/{i}"));
                    leaves(x, at, out);
                    at.truncate(len);
                }
            }
            Value::Object(m) => {
                for (k, x) in m {
                    at.push_str(&format!("/{}", k.replace('~', "~0").replace('/', "~1")));
                    leaves(x, at, out);
                    at.truncate(len);
                }
            }
            Value::Null => {}
        }
    }

    const NUMBERS: &[f64] = &[
        0.0, -1.0, 1.0, 0.5, 2.0, 3.0, 7.5, -7.5, 31.0, 32.0, 543.0, 544.0, 999.0, 1000.0, 1e9, -1e9, 1e12, 1e-9,
        12345.678,
    ];
    const STRINGS: &[&str] = &[
        "",
        "  lead",
        "tail\\",
        "a\r\nb",
        "x\u{1}y",
        "\\001",
        "é€ü",
        "line1\nline2",
        "\ttab",
        "a\\b\\",
        "# hash",
        " ",
        "back\\001slash",
        "\nlead newline",
        "\u{0}",
    ];

    /// A document with a few random fields of its JSON changed: hostile but
    /// shaped like one (sizes zero or negative, texts with odd characters).
    fn mutate(doc: &Document, rng: &mut Rng) -> Option<Document> {
        let mut v = serde_json::to_value(doc).ok()?;
        let mut ls = Vec::new();
        leaves(&v, &mut String::new(), &mut ls);
        for _ in 0..1 + rng.below(4) {
            let path = ls.get(rng.below(ls.len()))?;
            let leaf = v.pointer_mut(path)?;
            match leaf {
                Value::Number(_) => *leaf = serde_json::json!(NUMBERS[rng.below(NUMBERS.len())]),
                Value::String(_) => *leaf = Value::String(STRINGS[rng.below(STRINGS.len())].to_string()),
                Value::Bool(b) => *b = !*b,
                _ => {}
            }
        }
        serde_json::from_value(v).ok()
    }

    /// Fixtures and sample figures with random fields changed: through
    /// objects or text, the output is the same (or both fail the same way).
    #[test]
    fn hostile_documents_export_identically() {
        set_clock();
        let dir = root().join("crates/engine/tests/fixtures");
        let mut docs = Vec::new();
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "fig") {
                docs.extend(load(&p));
            }
        }
        let cases: usize = std::env::var("D7_CASES")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(300);
        let langs = [
            "eps", "svg", "pdf", "tikz", "latex", "pstricks", "box", "ps", "pic", "mf", "emf", "png",
        ];
        let sets = option_sets();
        let mut rng = Rng(std::env::var("D7_SEED")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(20261020));
        let (mut n, mut used, mut tried) = (0, 0, 0);
        for k in 0..cases {
            let Some(base) = docs.get(rng.below(docs.len())) else {
                break;
            };
            let Some(doc) = mutate(base, &mut rng) else { continue };
            tried += 1;
            used += usize::from(objects_used(&doc));
            for lang in langs.iter().filter(|l| driver_available(l)) {
                check(&doc, lang, &sets[(k + n) % sets.len()], &format!("hostile case {k}"));
                n += 1;
            }
        }
        eprintln!("hostile: {tried} documents ({used} through objects), {n} exports identical");
    }
}
