//! `cargo xtask examples [OUTDIR]`: writes the figures the project makes from
//! code (port of the two Python generators this replaces) as Fig 3.2 files
//! under `examples/` (or OUTDIR):
//!
//! - `gallery/<name>.fig`, the 16 scientific drawings (physics and
//!   mathematics, LaTeX special text) of the science gallery;
//! - `<name>/<name>.fig.gz` for four figures generated from mathematics: the
//!   Clifford attractor, the Apollonian gasket, the Penrose rhomb tiling and
//!   the Gosper curve.
//!
//! The output is deterministic (the same bytes on every run, on every machine
//! that has the same floating-point maths library): neither random numbers
//! other than a fixed-seed generator, nor LaTeX, nor the clock take part.

mod gallery;
mod math;
mod metrics;
mod pyish;

use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use std::path::Path;

/// One output file: path relative to the output directory, bytes.
type Output = (String, Vec<u8>);

fn gzip(text: &str) -> Result<Vec<u8>, String> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::best());
    enc.write_all(text.as_bytes()).map_err(|e| format!("gzip: {e}"))?;
    enc.finish().map_err(|e| format!("gzip: {e}"))
}

/// Every file `cargo xtask examples` writes.
fn outputs() -> Result<Vec<Output>, String> {
    let mut out: Vec<Output> = Vec::new();
    for (name, text) in gallery::all()? {
        out.push((format!("gallery/{name}.fig"), text.into_bytes()));
    }
    for (name, text) in math::all() {
        out.push((format!("{name}/{name}.fig.gz"), gzip(&text)?));
    }
    Ok(out)
}

/// Writes all figures below `dir`; returns the number of files.
fn write_all(dir: &Path) -> Result<usize, String> {
    let files = outputs()?;
    for (rel, bytes) in &files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(files.len())
}

pub fn run(root: &Path, outdir: Option<String>) -> Result<(), String> {
    let dir = outdir.map_or_else(|| root.join("examples"), std::path::PathBuf::from);
    let n = write_all(&dir)?;
    println!("examples: wrote {n} figures to {}", dir.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("rustfig-examples-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn generator_is_deterministic() {
        // two of the figures, written twice into separate directories
        let (a, b) = (temp_dir("a"), temp_dir("b"));
        for dir in [&a, &b] {
            let files = outputs().unwrap();
            for name in ["gallery/loglog.fig", "gosper-curve/gosper-curve.fig.gz"] {
                let (rel, bytes) = files.iter().find(|(r, _)| r == name).unwrap();
                let path = dir.join(rel);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, bytes).unwrap();
            }
        }
        for name in ["gallery/loglog.fig", "gosper-curve/gosper-curve.fig.gz"] {
            assert_eq!(
                std::fs::read(a.join(name)).unwrap(),
                std::fs::read(b.join(name)).unwrap(),
                "{name}"
            );
        }
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
    }

    #[test]
    fn writes_all_twenty_figures_with_valid_headers() {
        let files = outputs().unwrap();
        assert_eq!(files.len(), 20);
        for (rel, bytes) in &files {
            let text = if rel.ends_with(".gz") {
                use std::io::Read;
                let mut s = String::new();
                flate2::read::GzDecoder::new(&bytes[..]).read_to_string(&mut s).unwrap();
                s
            } else {
                String::from_utf8(bytes.clone()).unwrap()
            };
            assert!(text.starts_with("#FIG 3.2"), "{rel}");
        }
    }
}
