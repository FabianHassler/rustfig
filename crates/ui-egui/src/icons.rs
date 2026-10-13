//! Tool icons, drawn in code.

use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Shape, Stroke};
use rustfig_engine::ToolKind;

fn pts(r: Rect, list: &[(f32, f32)]) -> Vec<Pos2> {
    list.iter()
        .map(|(x, y)| pos2(r.left() + x * r.width(), r.top() + y * r.height()))
        .collect()
}

fn curve(r: Rect, closed: bool) -> Vec<Pos2> {
    let n = 24;
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            if closed {
                let a = t * std::f32::consts::TAU;
                pos2(
                    r.center().x + 0.38 * r.width() * a.cos() * (1.0 + 0.2 * (2.0 * a).sin()),
                    r.center().y + 0.3 * r.height() * a.sin(),
                )
            } else {
                pos2(
                    r.left() + (0.1 + 0.8 * t) * r.width(),
                    r.center().y - 0.3 * r.height() * (t * std::f32::consts::TAU).sin(),
                )
            }
        })
        .collect()
}

/// Paint the icon for `tool` inside `r`.
pub fn paint(p: &Painter, r: Rect, tool: ToolKind, color: Color32) {
    let s = Stroke::new(1.6_f32, color);
    let thin = Stroke::new(1.0_f32, color);
    let c = r.center();
    let w = r.width();
    let dot = |p: &Painter, q: Pos2| p.circle_filled(q, 1.8, color);
    match tool {
        ToolKind::Select => {
            p.add(Shape::convex_polygon(
                pts(
                    r,
                    &[
                        (0.3, 0.15),
                        (0.3, 0.8),
                        (0.45, 0.65),
                        (0.58, 0.9),
                        (0.66, 0.86),
                        (0.54, 0.6),
                        (0.75, 0.6),
                    ],
                ),
                color,
                Stroke::NONE,
            ));
        }
        ToolKind::CircleRadius | ToolKind::CircleDiameter => {
            p.circle_stroke(c, w * 0.32, s);
            if tool == ToolKind::CircleRadius {
                p.line_segment([c, c + vec2(w * 0.32, 0.0)], thin);
            } else {
                p.line_segment([c - vec2(w * 0.32, 0.0), c + vec2(w * 0.32, 0.0)], thin);
            }
        }
        ToolKind::EllipseRadii | ToolKind::EllipseDiameter => {
            let e: Vec<Pos2> = (0..=32)
                .map(|i| {
                    let a = i as f32 / 32.0 * std::f32::consts::TAU;
                    c + vec2(0.38 * w * a.cos(), 0.24 * w * a.sin())
                })
                .collect();
            p.add(Shape::line(e, s));
            if tool == ToolKind::EllipseRadii {
                p.line_segment([c, c + vec2(w * 0.38, 0.0)], thin);
                p.line_segment([c, c - vec2(0.0, w * 0.24)], thin);
            } else {
                p.line_segment([c - vec2(w * 0.38, 0.0), c + vec2(w * 0.38, 0.0)], thin);
            }
        }
        ToolKind::SplineApprox | ToolKind::SplineInterp | ToolKind::XSpline => {
            p.add(Shape::line(curve(r, false), s));
            if tool != ToolKind::SplineApprox {
                for q in pts(r, &[(0.1, 0.5), (0.35, 0.2), (0.65, 0.8), (0.9, 0.5)]) {
                    dot(p, q);
                }
            }
            if tool == ToolKind::XSpline {
                p.text(
                    r.right_bottom() - vec2(3.0, 3.0),
                    egui::Align2::RIGHT_BOTTOM,
                    "x",
                    egui::FontId::proportional(9.0),
                    color,
                );
            }
        }
        ToolKind::SplineApproxClosed | ToolKind::SplineInterpClosed | ToolKind::XSplineClosed => {
            p.add(Shape::closed_line(curve(r, true), s));
            if tool != ToolKind::SplineApproxClosed {
                dot(p, c + vec2(w * 0.38, 0.0));
                dot(p, c - vec2(w * 0.38, 0.0));
            }
            if tool == ToolKind::XSplineClosed {
                p.text(
                    r.right_bottom() - vec2(3.0, 3.0),
                    egui::Align2::RIGHT_BOTTOM,
                    "x",
                    egui::FontId::proportional(9.0),
                    color,
                );
            }
        }
        ToolKind::Polygon => {
            p.add(Shape::closed_line(
                pts(r, &[(0.2, 0.75), (0.3, 0.25), (0.75, 0.2), (0.85, 0.6), (0.5, 0.85)]),
                s,
            ));
        }
        ToolKind::Polyline => {
            p.add(Shape::line(
                pts(r, &[(0.15, 0.8), (0.35, 0.25), (0.6, 0.7), (0.85, 0.2)]),
                s,
            ));
        }
        ToolKind::Box | ToolKind::Picture => {
            let b = r.shrink(w * 0.22);
            p.rect_stroke(b, 0.0, s, egui::StrokeKind::Middle);
            if tool == ToolKind::Picture {
                p.add(Shape::line(
                    pts(r, &[(0.25, 0.72), (0.45, 0.45), (0.58, 0.6), (0.68, 0.5), (0.76, 0.72)]),
                    thin,
                ));
                p.circle_filled(
                    pos2(b.left() + b.width() * 0.25, b.top() + b.height() * 0.3),
                    2.0,
                    color,
                );
            }
        }
        ToolKind::ArcBox => {
            p.rect_stroke(r.shrink(w * 0.22), w * 0.12, s, egui::StrokeKind::Middle);
        }
        ToolKind::RegularPolygon => {
            let v: Vec<Pos2> = (0..6)
                .map(|i| {
                    let a = i as f32 / 6.0 * std::f32::consts::TAU;
                    c + vec2(0.34 * w * a.cos(), 0.34 * w * a.sin())
                })
                .collect();
            p.add(Shape::closed_line(v, s));
        }
        ToolKind::Arc => {
            let a: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let t = std::f32::consts::PI * (1.0 + i as f32 / 20.0);
                    c + vec2(0.34 * w * t.cos(), 0.34 * w * t.sin() + 0.12 * w)
                })
                .collect();
            p.add(Shape::line(a, s));
        }
        ToolKind::Text => {
            p.text(
                c,
                egui::Align2::CENTER_CENTER,
                "T",
                egui::FontId::proportional(w * 0.6),
                color,
            );
        }
        ToolKind::Glue | ToolKind::Break => {
            let a = Rect::from_min_size(r.min + vec2(w * 0.15, w * 0.15), vec2(w * 0.35, w * 0.35));
            let b = Rect::from_min_size(r.min + vec2(w * 0.5, w * 0.5), vec2(w * 0.35, w * 0.35));
            p.rect_stroke(a, 0.0, thin, egui::StrokeKind::Middle);
            p.circle_stroke(b.center(), w * 0.17, thin);
            let outer = r.shrink(w * 0.08);
            if tool == ToolKind::Glue {
                p.rect_stroke(outer, 2.0, s, egui::StrokeKind::Middle);
            } else {
                let c = [
                    outer.left_top(),
                    outer.right_top(),
                    outer.right_bottom(),
                    outer.left_bottom(),
                    outer.left_top(),
                ];
                p.extend(Shape::dashed_line(&c, thin, 3.0, 3.0));
            }
        }
        ToolKind::Scale => {
            p.rect_stroke(
                Rect::from_min_size(r.min + vec2(w * 0.15, w * 0.45), vec2(w * 0.4, w * 0.4)),
                0.0,
                thin,
                egui::StrokeKind::Middle,
            );
            p.rect_stroke(r.shrink(w * 0.15), 0.0, thin, egui::StrokeKind::Middle);
            arrow(p, c, r.right_top() + vec2(-w * 0.2, w * 0.2), s);
        }
        ToolKind::Align => {
            let x = r.left() + w * 0.2;
            p.line_segment([pos2(x, r.top() + w * 0.1), pos2(x, r.bottom() - w * 0.1)], s);
            for (y, len) in [(0.25, 0.6), (0.5, 0.35), (0.75, 0.5)] {
                let yy = r.top() + w * y;
                p.line_segment([pos2(x, yy), pos2(x + w * len, yy)], Stroke::new(3.0_f32, color));
            }
        }
        ToolKind::Move | ToolKind::Copy => {
            if tool == ToolKind::Copy {
                p.rect_stroke(
                    Rect::from_min_size(r.min + vec2(w * 0.12, w * 0.12), vec2(w * 0.4, w * 0.4)),
                    0.0,
                    thin,
                    egui::StrokeKind::Middle,
                );
            }
            for d in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                arrow(p, c, c + d * w * 0.38, s);
            }
        }
        ToolKind::MovePoint | ToolKind::AddPoint | ToolKind::DeletePoint => {
            let l = pts(r, &[(0.1, 0.8), (0.45, 0.35), (0.9, 0.7)]);
            p.add(Shape::line(l.clone(), thin));
            let v = l.get(1).copied().unwrap_or(c);
            p.rect_filled(Rect::from_center_size(v, vec2(5.0, 5.0)), 0.0, color);
            match tool {
                ToolKind::MovePoint => arrow(p, v, v + vec2(w * 0.3, -w * 0.25), thin),
                ToolKind::AddPoint => {
                    let q = r.right_top() + vec2(-w * 0.2, w * 0.2);
                    p.line_segment([q - vec2(4.0, 0.0), q + vec2(4.0, 0.0)], s);
                    p.line_segment([q - vec2(0.0, 4.0), q + vec2(0.0, 4.0)], s);
                }
                _ => {
                    let q = r.right_top() + vec2(-w * 0.2, w * 0.2);
                    p.line_segment([q - vec2(4.0, 0.0), q + vec2(4.0, 0.0)], s);
                }
            }
        }
        ToolKind::Delete => {
            let b = r.shrink(w * 0.25);
            p.line_segment([b.left_top(), b.right_bottom()], Stroke::new(2.2_f32, color));
            p.line_segment([b.right_top(), b.left_bottom()], Stroke::new(2.2_f32, color));
        }
        ToolKind::Update | ToolKind::Edit => {
            p.rect_stroke(r.shrink(w * 0.2), 0.0, thin, egui::StrokeKind::Middle);
            if tool == ToolKind::Edit {
                p.line_segment(
                    [
                        pos2(r.left() + w * 0.3, r.bottom() - w * 0.3),
                        pos2(r.right() - w * 0.15, r.top() + w * 0.15),
                    ],
                    Stroke::new(2.5_f32, color),
                );
            } else {
                p.rect_filled(
                    Rect::from_min_size(r.min + vec2(w * 0.2, w * 0.2), vec2(w * 0.3, w * 0.3)),
                    0.0,
                    color,
                );
                arrow(
                    p,
                    pos2(r.left() + w * 0.35, r.top() + w * 0.35),
                    pos2(r.right() - w * 0.25, r.bottom() - w * 0.25),
                    thin,
                );
            }
        }
        ToolKind::FlipV | ToolKind::FlipH => {
            let (a, b) = if tool == ToolKind::FlipH {
                (pos2(c.x, r.top() + 2.0), pos2(c.x, r.bottom() - 2.0))
            } else {
                (pos2(r.left() + 2.0, c.y), pos2(r.right() - 2.0, c.y))
            };
            p.extend(Shape::dashed_line(&[a, b], thin, 2.0, 2.0));
            let tri = if tool == ToolKind::FlipH {
                [(0.15, 0.75), (0.42, 0.75), (0.42, 0.25)]
            } else {
                [(0.25, 0.15), (0.25, 0.42), (0.75, 0.42)]
            };
            p.add(Shape::convex_polygon(pts(r, &tri), color, Stroke::NONE));
            let mirrored: Vec<(f32, f32)> = if tool == ToolKind::FlipH {
                tri.iter().map(|(x, y)| (1.0 - x, *y)).collect()
            } else {
                tri.iter().map(|(x, y)| (*x, 1.0 - y)).collect()
            };
            p.add(Shape::closed_line(pts(r, &mirrored), thin));
        }
        ToolKind::RotateCcw | ToolKind::RotateCw => {
            let n = 20;
            let arc: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let a = std::f32::consts::PI * (0.2 + 1.5 * i as f32 / n as f32);
                    c + vec2(w * 0.32 * a.cos(), -w * 0.32 * a.sin())
                })
                .collect();
            let (end, prev) = if tool == ToolKind::RotateCcw {
                (arc.last().copied().unwrap_or(c), arc.get(n - 2).copied().unwrap_or(c))
            } else {
                (arc.first().copied().unwrap_or(c), arc.get(2).copied().unwrap_or(c))
            };
            p.add(Shape::line(arc, s));
            arrow(p, prev, end, s);
        }
        ToolKind::Convert => {
            p.add(Shape::line(pts(r, &[(0.1, 0.4), (0.3, 0.15), (0.5, 0.4)]), thin));
            p.add(Shape::line(
                curve(
                    Rect::from_min_size(r.min + vec2(w * 0.3, w * 0.4), vec2(w * 0.65, w * 0.5)),
                    false,
                ),
                s,
            ));
        }
        ToolKind::OpenClose => {
            let n = 24;
            let arc: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let a = std::f32::consts::TAU * (0.1 + 0.8 * i as f32 / n as f32);
                    c + vec2(w * 0.32 * a.cos(), w * 0.32 * a.sin())
                })
                .collect();
            p.add(Shape::line(arc, s));
        }
        ToolKind::Arrow => {
            arrow(p, pos2(r.left() + w * 0.15, c.y), pos2(r.right() - w * 0.15, c.y), s);
        }
    }
}

/// A line with an arrowhead at `to`.
fn arrow(p: &Painter, from: Pos2, to: Pos2, s: Stroke) {
    p.line_segment([from, to], s);
    let d = (to - from).normalized();
    let n = vec2(-d.y, d.x);
    let back = to - d * 5.0;
    p.add(Shape::convex_polygon(
        vec![to, back + n * 3.0, back - n * 3.0],
        s.color,
        Stroke::NONE,
    ));
}
