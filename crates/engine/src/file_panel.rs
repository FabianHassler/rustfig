//! What xfig's Open and Merge panel shows besides the file name (W10-12):
//! the files of a directory that match the filename mask, the selected
//! figure's size, object count and comments, a small preview of it, and
//! the Figure Offset of Merge.
//!
// Ported from xfig src/w_file.c (`file_panel_dismiss`, `file_getxyoff`, the
// mask and directory lists, `preview_figure`, `clear_preview`,
// `check_cancel`) (xfig licence, see LICENSE-xfig).

use crate::{Engine, EngineError};
use rustfig_geom::{Affine, Rect};
use rustfig_model::{Document, Units};
use rustfig_render::raster::{rasterize, Pixmap, RasterOptions};
use rustfig_render::SceneOptions;
use serde_json::{json, Value};
use std::path::Path;

/// The most entries of one directory the panel lists.
pub const MAX_ENTRIES: usize = 20_000;
/// xfig's default filename mask, plus the figures RustFig opens.
pub const DEFAULT_MASK: &str = "*.fig *.rustfig *.xfig *.fig.gz *.fig.Z *.pdf";

/// Whether `name` matches the glob `pattern` (`*`, `?` and `[set]`, as the
/// mask of xfig's file list uses).
pub fn glob_match(pattern: &str, name: &str) -> bool {
    fn go(p: &[char], n: &[char]) -> bool {
        match p.split_first() {
            None => n.is_empty(),
            Some(('*', rest)) => (0..=n.len()).any(|i| n.get(i..).is_some_and(|tail| go(rest, tail))),
            Some(('?', rest)) => n.split_first().is_some_and(|(_, tail)| go(rest, tail)),
            Some(('[', rest)) => {
                let Some(close) = rest.iter().position(|c| *c == ']') else {
                    // No closing bracket: a literal.
                    return n.split_first().is_some_and(|(c, tail)| *c == '[' && go(rest, tail));
                };
                let (set, after) = (&rest[..close], &rest[close + 1..]);
                let Some((c, tail)) = n.split_first() else {
                    return false;
                };
                let (neg, set) = match set.split_first() {
                    Some(('!' | '^', s)) => (true, s),
                    _ => (false, set),
                };
                let mut hit = false;
                let mut i = 0;
                while i < set.len() {
                    if set.get(i + 1) == Some(&'-') && i + 2 < set.len() {
                        if set[i] <= *c && *c <= set[i + 2] {
                            hit = true;
                        }
                        i += 3;
                    } else {
                        hit |= set[i] == *c;
                        i += 1;
                    }
                }
                hit != neg && go(after, tail)
            }
            Some((c, rest)) => n.split_first().is_some_and(|(x, tail)| x == c && go(rest, tail)),
        }
    }
    // Patterns are capped so a hostile mask cannot make the matcher crawl.
    if pattern.chars().count() > 256 {
        return false;
    }
    let (p, n): (Vec<char>, Vec<char>) = (pattern.chars().collect(), name.chars().collect());
    go(&p, &n)
}

/// Whether `name` matches any of the blank-separated patterns of `mask`
/// (an empty mask matches everything).
pub fn mask_match(mask: &str, name: &str) -> bool {
    let mut any = false;
    for pat in mask.split_whitespace() {
        any = true;
        if glob_match(pat, name) {
            return true;
        }
    }
    !any
}

/// The listing of a directory: its sub-directories (first `..`) and the
/// files that match the mask, each sorted by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirListing {
    pub dir: String,
    pub dirs: Vec<String>,
    pub files: Vec<String>,
}

/// List `dir` (`Rescan`): the files matching `mask`, hidden ones only with
/// `show_hidden` (xfig's "Show Hidden" button).
pub fn list_dir(dir: &Path, mask: &str, show_hidden: bool) -> Result<DirListing, EngineError> {
    let read = std::fs::read_dir(dir).map_err(|e| EngineError::io(format!("{}: {e}", dir.display())))?;
    let mut dirs = vec!["..".to_string()];
    let mut files = Vec::new();
    for entry in read.flatten().take(MAX_ENTRIES) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        // `metadata` follows links, so a link to a directory is a directory.
        let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
        if is_dir {
            dirs.push(name);
        } else if mask_match(mask, &name) {
            files.push(name);
        }
    }
    dirs[1..].sort_by_key(|s| s.to_lowercase());
    files.sort_by_key(|s| s.to_lowercase());
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    Ok(DirListing {
        dir: dir.to_string_lossy().into_owned(),
        dirs,
        files,
    })
}

/// A figure read for the panel.
pub struct FigurePreview {
    pub objects: usize,
    /// The figure's size in Fig units.
    pub width: f64,
    pub height: f64,
    /// "3.2x3.2 in" (`preview_figure`'s size label), in the unit asked for.
    pub size: String,
    pub comments: String,
    doc: Document,
    bounds: Rect,
}

impl FigurePreview {
    /// The JSON the `file.preview` command returns.
    pub fn json(&self) -> Value {
        json!({
            "objects": self.objects,
            "width": self.width,
            "height": self.height,
            "size": self.size,
            "comments": self.comments,
        })
    }

    /// The figure drawn to fit in `max_w` x `max_h` pixels on white
    /// (RGBA, row by row); `None` when the figure is empty.
    /// The preview's scene: special (LaTeX) text as Unicode, as in the
    /// library previews (no TeX run for a preview).
    fn scene(&self) -> rustfig_render::Scene {
        rustfig_render::build_scene(
            &self.doc,
            &SceneOptions {
                approx_latex: true,
                ..SceneOptions::default()
            },
        )
    }

    pub fn render(&self, max_w: u32, max_h: u32) -> Option<(u32, u32, Vec<u8>)> {
        let b = self.bounds;
        if b.is_empty() || b.width() <= 0.0 || b.height() <= 0.0 || max_w == 0 || max_h == 0 {
            return None;
        }
        let (max_w, max_h) = (max_w.min(2048), max_h.min(2048));
        let k = (f64::from(max_w) / b.width()).min(f64::from(max_h) / b.height());
        if !k.is_finite() || k <= 0.0 {
            return None;
        }
        let (w, h) = (
            ((b.width() * k).ceil() as u32).clamp(1, max_w),
            ((b.height() * k).ceil() as u32).clamp(1, max_h),
        );
        let scene = self.scene();
        let mut pm = Pixmap::new(w, h)?;
        for px in pm.data_mut().as_chunks_mut::<4>().0 {
            *px = [255, 255, 255, 255];
        }
        rasterize(
            &scene,
            &mut pm,
            &RasterOptions {
                transform: Affine::translate(-b.min.x, -b.min.y).then(&Affine::scale(k, k)),
                min_line_px: 1.0,
                adjust_px: 0.0,
                pattern_k: 1,
                pattern: rustfig_render::pattern::PatternFrame::screen(),
            },
        );
        Some((w, h, pm.data().to_vec()))
    }
}

/// Read `path` for the panel: size, number of objects, comments and the
/// figure itself for [`FigurePreview::render`] (`preview_figure`). The size
/// is in `units`, the open figure's, as xfig's `appres.INCHES`.
pub fn preview_figure(units: Units, path: &str) -> Result<FigurePreview, EngineError> {
    let (mut doc, _) = crate::cmd::file::read_figure(path)?;
    let objects = doc.objects.len();
    let comments = doc.comments.join("\n");
    doc.open.clear();
    // A scratch session measures the figure the way the canvas does.
    let mut scratch = Engine::new();
    scratch.replace_document(doc, None);
    let bounds = scratch.doc_bounds();
    let (width, height) = if bounds.is_empty() {
        (0.0, 0.0)
    } else {
        (bounds.width(), bounds.height())
    };
    let (per, name) = match units {
        Units::Inches => (1200.0, "in"),
        Units::Metric => (450.0, "cm"),
    };
    let size = format!("{:.1}x{:.1} {name}", width / per, height / per);
    Ok(FigurePreview {
        objects,
        width,
        height,
        size,
        comments,
        doc: scratch.doc().clone(),
        bounds,
    })
}

/// `file_getxyoff`: the Figure Offset typed in `value` of `unit` (0 inches,
/// 1 cm, 2 Fig units) in Fig units.
pub fn offset_in_fig_units(value: f64, unit: u8) -> f64 {
    let per = match unit {
        0 => 1200.0,
        1 => 450.0,
        _ => 1.0,
    };
    let v = value * per;
    if v.is_finite() {
        v.clamp(-1.0e7, 1.0e7)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_like_xfig() {
        assert!(glob_match("*.fig", "illusion.fig"));
        assert!(!glob_match("*.fig", "illusion.fig.gz"));
        assert!(mask_match(DEFAULT_MASK, "a.fig.gz"));
        assert!(glob_match("*.fig.[Z]", "a.fig.Z") && !glob_match("*.fig.[Z]", "a.fig.z"));
        assert!(glob_match("h?use*", "house_elev.fig"));
        assert!(glob_match("[a-c]*", "b.fig") && !glob_match("[!a-c]*", "b.fig"));
        assert!(mask_match("", "anything") && !mask_match("*.fig", "x.png"));
        assert!(!glob_match("[", "x"));
    }

    #[test]
    fn preview_shows_latex_text_approximately() {
        let dir = std::env::temp_dir().join(format!("rustfig-fp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("latex.fig");
        std::fs::write(
            &path,
            "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
             4 0 0 50 -1 0 12 0.0000 2 135 450 600 600 $\\\\lambda/2$\\001\n",
        )
        .unwrap();
        let fig = preview_figure(Units::Inches, path.to_str().unwrap()).unwrap();
        let texts: Vec<String> = fig
            .scene()
            .prims
            .iter()
            .filter_map(|p| match p {
                rustfig_render::Prim::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, ["λ/2"]);
        assert!(fig.render(64, 64).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn offsets_follow_the_unit() {
        assert_eq!(offset_in_fig_units(1.0, 0), 1200.0);
        assert_eq!(offset_in_fig_units(2.0, 1), 900.0);
        assert_eq!(offset_in_fig_units(15.0, 2), 15.0);
        assert_eq!(offset_in_fig_units(f64::NAN, 0), 0.0);
    }
}
