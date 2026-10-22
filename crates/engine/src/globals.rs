//! The rows of xfig's Global settings panel that act on the drawing window
//! (`show_global_settings`, `w_cmdpanel.c`), read from the preferences
//! (`prefs.set` changes them): the canvas, the rulers and the tools ask here.

use crate::Engine;
use rustfig_geom::Point;
use rustfig_model::{Object, ObjectKind};

/// xfig's `DEF_FREEHAND_RES`: points of a freehand stroke are this many
/// Fig units (at zoom 1, in pixels) apart.
pub const DEFAULT_FREEHAND_RESOLUTION: i64 = 25;
/// Cap on the vertex numbers drawn at once.
pub const MAX_VERTEX_NUMBERS: usize = 5000;

impl Engine {
    fn pref_bool(&self, name: &str, default: bool) -> bool {
        self.prefs.bool(name).unwrap_or(default)
    }

    /// "Track mouse in rulers": the rulers' markers follow the pointer.
    pub fn track_mouse(&self) -> bool {
        self.pref_bool("trackCursor", true)
    }

    /// "Allow negative coords": points left of or above the origin.
    pub fn allow_negative_coords(&self) -> bool {
        self.pref_bool("allownegcoords", true)
    }

    /// "Draw axis lines": the lines through the origin.
    pub fn show_axis_lines(&self) -> bool {
        self.pref_bool("showaxislines", true)
    }

    /// "Show vertex numbers": the index of each point of lines and splines.
    pub fn show_vertex_numbers(&self) -> bool {
        self.pref_bool("shownums", false)
    }

    /// "Freehand drawing resolution" (xfig's `freehand_resolution`), at least 1.
    pub fn freehand_resolution(&self) -> f64 {
        self.prefs
            .int("freehand_resolution")
            .unwrap_or(DEFAULT_FREEHAND_RESOLUTION)
            .clamp(1, 10_000) as f64
    }

    /// Keep a point out of the negative quadrant unless negative coordinates
    /// are allowed.
    pub fn limit_point(&self, p: Point) -> Point {
        if self.allow_negative_coords() {
            p
        } else {
            Point::new(p.x.max(0.0), p.y.max(0.0))
        }
    }

    /// The numbers xfig draws at the vertices, in Fig units: the point and
    /// its number (counted from 1) for polylines, polygons and splines
    /// (also inside compounds).
    pub fn vertex_numbers(&self) -> Vec<(Point, usize)> {
        let mut out = Vec::new();
        for o in &self.doc().objects {
            collect_numbers(o, &mut out);
            if out.len() >= MAX_VERTEX_NUMBERS {
                break;
            }
        }
        out.truncate(MAX_VERTEX_NUMBERS);
        out
    }
}

fn collect_numbers(o: &Object, out: &mut Vec<(Point, usize)>) {
    match &o.kind {
        ObjectKind::Polyline(l) => out.extend(l.points.iter().enumerate().map(|(i, p)| (*p, i + 1))),
        ObjectKind::Spline(s) => out.extend(s.points.iter().enumerate().map(|(i, p)| (*p, i + 1))),
        ObjectKind::Compound(c) => {
            for m in &c.objects {
                if out.len() >= MAX_VERTEX_NUMBERS {
                    break;
                }
                collect_numbers(m, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_are_xfigs_and_prefs_change_them() {
        let mut e = Engine::new();
        assert!(e.track_mouse() && e.allow_negative_coords() && e.show_axis_lines());
        assert!(!e.show_vertex_numbers());
        assert_eq!(e.freehand_resolution(), 25.0);
        for (name, v) in [
            ("trackCursor", json!(false)),
            ("allownegcoords", json!(false)),
            ("showaxislines", json!(false)),
            ("shownums", json!(true)),
            ("freehand_resolution", json!(60)),
        ] {
            e.execute("prefs.set", &json!({"resource": name, "value": v})).unwrap();
        }
        assert!(!e.track_mouse() && !e.allow_negative_coords() && !e.show_axis_lines());
        assert!(e.show_vertex_numbers());
        assert_eq!(e.freehand_resolution(), 60.0);
        assert_eq!(e.limit_point(Point::new(-5.0, 7.0)), Point::new(0.0, 7.0));
    }

    #[test]
    fn vertex_numbers_follow_the_points() {
        let mut e = Engine::new();
        e.execute("shape.polyline", &json!({"points": [[0, 0], [100, 0], [100, 100]]}))
            .unwrap();
        let n = e.vertex_numbers();
        assert_eq!(n.len(), 3);
        assert_eq!(n.last().map(|x| x.1), Some(3));
    }
}
