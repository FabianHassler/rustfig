//! The mode panel's Go to mode field and Favorites strip: type part of a
//! mode's name to pick it, keep the modes you use most within reach.

use crate::panels;
use crate::theme::Tokens;
use crate::App;
use egui::{vec2, Align2, FontId, Key, Modifiers, Sense, TextEdit};
use rustfig_engine::ToolKind;
use serde_json::json;

/// Most matches listed.
const MAX_MATCHES: usize = 10;
/// Size of a Favorites button.
const FAV_SIZE: egui::Vec2 = vec2(17.0, 16.0);

/// State of the Go to mode field.
#[derive(Default)]
pub struct ModeFinder {
    /// What was typed.
    pub text: String,
    /// The highlighted match.
    pub sel: usize,
    /// Put the caret in the field on the next frame (Go to mode command).
    pub focus: bool,
    /// The field had the caret last frame.
    pub active: bool,
    /// Escape was pressed while it was active (taken by `handle_shortcuts`,
    /// since egui drops the focus on Escape first).
    pub escape: bool,
    dismissed: bool,
}

/// The text a mode is found by: its name and xfig's long text without the key.
fn haystack(t: ToolKind) -> (String, String) {
    let label = t.label().to_lowercase();
    let long = t
        .xfig_message()
        .map(|m| m.rfind('(').map_or(m, |i| m.split_at(i).0).trim().to_lowercase())
        .unwrap_or_default();
    (label, long)
}

/// The modes whose name or long text holds every word of `query`
/// (case-insensitive): names starting with the query first, then names
/// containing it, then long texts; each in the panel's order.
pub fn matches(query: &str) -> Vec<ToolKind> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(u8, ToolKind)> = Vec::new();
    // The panel's modes, then those without a button (the X-splines, the
    // ellipse modes and Open/close), which are found only here.
    let modes = panels::panel_tools().into_iter().chain([
        ToolKind::XSpline,
        ToolKind::XSplineClosed,
        ToolKind::EllipseRadii,
        ToolKind::EllipseDiameter,
        ToolKind::OpenClose,
    ]);
    for t in modes {
        let (label, long) = haystack(t);
        let all_in = |s: &str| words.iter().all(|w| s.contains(w.as_str()));
        let rank = if words.len() == 1 && label.starts_with(words[0].as_str()) {
            0
        } else if all_in(&label) {
            1
        } else if all_in(&format!("{label} {long}")) {
            2
        } else {
            continue;
        };
        hits.push((rank, t));
    }
    hits.sort_by_key(|(r, _)| *r);
    hits.into_iter().map(|(_, t)| t).take(MAX_MATCHES).collect()
}

/// A mode's keys as the balloon shows them for this system, "" when it has none.
pub fn mode_keys(t: ToolKind, mac: bool) -> String {
    let tip = panels::tip_text(t, mac);
    match (tip.rfind('('), tip.ends_with(')')) {
        (Some(i), true) => tip.get(i + 1..tip.len() - 1).unwrap_or("").to_string(),
        _ => String::new(),
    }
}

/// The favorite modes that have a button, in the order they were added.
pub fn favorites(app: &App) -> Vec<ToolKind> {
    let panel = panels::panel_tools();
    app.engine
        .prefs
        .rustfig
        .mode_favourites
        .iter()
        .filter_map(|id| ToolKind::from_id(id))
        .filter(|t| panel.contains(t))
        .collect()
}

/// The finder and the Favorites strip, above the scrolling buttons.
pub fn show(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    finder(app, ui, tk);
    strip(app, ui, tk);
}

fn finder(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    let ctx = ui.ctx().clone();
    let id = egui::Id::new("mode_finder_field");
    let mut f = std::mem::take(&mut app.mode_finder);
    let hits = if f.dismissed { Vec::new() } else { matches(&f.text) };
    let had_focus = ctx.memory(|m| m.has_focus(id));
    let mut pick = None;
    let mut leave = std::mem::take(&mut f.escape) || (f.active && ctx.input(|i| i.key_pressed(Key::Escape)));
    if had_focus {
        // Consumed before the field sees them: Return would end the edit.
        if !hits.is_empty() {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                f.sel = (f.sel + 1) % hits.len();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
                f.sel = (f.sel + hits.len() - 1) % hits.len();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                pick = hits.get(f.sel.min(hits.len() - 1)).copied();
            }
        }
    }
    let before = f.text.clone();
    let resp = ui.add(
        TextEdit::singleline(&mut f.text)
            .id(id)
            .hint_text("Go to mode")
            .font(FontId::proportional(11.0))
            .margin(vec2(3.0, 2.0))
            .desired_width(ui.available_width()),
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Go to mode"));
    f.active = ctx.memory(|m| m.has_focus(id)) && !leave;
    if f.focus {
        f.focus = false;
        resp.request_focus();
    }
    if f.text != before {
        f.sel = 0;
        f.dismissed = false;
    }
    let mut clicked = None;
    let open = !hits.is_empty() && !leave && pick.is_none();
    if open {
        let mac = crate::platform::Os::current(&ctx) == crate::platform::Os::Mac;
        let area = egui::Area::new(egui::Id::new("mode_finder_list"))
            .order(egui::Order::Foreground)
            .fixed_pos(resp.rect.left_bottom() + vec2(0.0, 2.0))
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).inner_margin(2.0).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    // One highlight: the row under the pointer becomes the
                    // selection (Return then picks it), so the first match does
                    // not stay lit beside it. Rows are laid out before they are
                    // painted, to know the hovered one first.
                    let rows: Vec<_> = hits
                        .iter()
                        .map(|_| ui.allocate_exact_size(vec2(220.0, 18.0), Sense::click()))
                        .collect();
                    if let Some(i) = rows.iter().position(|(_, rr)| rr.hovered()) {
                        f.sel = i;
                    }
                    for (i, (t, (r, rr))) in hits.iter().zip(rows).enumerate() {
                        if i == f.sel {
                            ui.painter().rect_filled(r, 2.0, tk.tool_active_bg);
                        }
                        let ink = ui.visuals().text_color();
                        // The mode's icon from the mode panel, then its name.
                        if let Some(icon) = panels::icon_of(*t) {
                            let ir = egui::Rect::from_center_size(r.left_center() + vec2(11.0, 0.0), vec2(18.0, 16.0));
                            let ink = if i == f.sel { tk.icon_active } else { tk.icon };
                            panels::paint_mode_icon(ui, icon, ir, ink);
                        }
                        ui.painter().text(
                            r.left_center() + vec2(24.0, 0.0),
                            Align2::LEFT_CENTER,
                            t.label(),
                            FontId::proportional(12.0),
                            ink,
                        );
                        ui.painter().text(
                            r.right_center() - vec2(4.0, 0.0),
                            Align2::RIGHT_CENTER,
                            mode_keys(*t, mac),
                            FontId::proportional(11.0),
                            tk.muted,
                        );
                        let label = format!("Go to mode result {}", t.label());
                        rr.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
                        if rr.clicked() {
                            clicked = Some(*t);
                        }
                    }
                });
            });
        // A press elsewhere closes the list and clears the field.
        let away = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !resp.rect.contains(p) && !area.response.rect.contains(p))
        });
        if away {
            leave = true;
        }
    }
    if let Some(t) = pick.or(clicked) {
        app.run(t.id(), json!({}));
        leave = true;
    }
    if leave {
        f.text.clear();
        f.sel = 0;
        f.dismissed = false;
        f.active = false;
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
    app.mode_finder = f;
}

fn strip(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    let favs = favorites(app);
    if favs.is_empty() {
        return;
    }
    ui.add_space(2.0);
    ui.label(egui::RichText::new("Favorites").small().color(tk.muted));
    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
    ui.horizontal_wrapped(|ui| {
        for t in favs {
            let Some(icon) = panels::icon_of(t) else { continue };
            let (rect, resp) = ui.allocate_exact_size(FAV_SIZE, Sense::click());
            let active = app.engine.tool() == t;
            let state = crate::bevel::State::of(&resp, active, false);
            crate::bevel::paint(ui.painter(), rect, tk, state);
            let ink = if active { tk.icon_active } else { tk.icon };
            panels::paint_mode_icon(ui, icon, rect.shrink(1.0), ink);
            let label = format!("Favorite {}", t.label());
            resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, &label));
            let resp = resp.on_hover_text(t.label());
            if resp.clicked() {
                app.run(t.id(), json!({}));
            }
            panels::favorite_menu(app, &resp, t);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_name_and_long_text_case_insensitively() {
        let m = matches("ARR");
        assert!(m.contains(&ToolKind::Arrow));
        assert_eq!(matches("arrow"), vec![ToolKind::Arrow]);
        assert!(matches("rounded").contains(&ToolKind::ArcBox));
        assert!(matches("").is_empty());
        assert!(matches("   ").is_empty());
        assert!(matches("qqqq").is_empty());
    }

    #[test]
    fn name_prefix_comes_first_and_x_splines_are_found_too() {
        let m = matches("box");
        assert_eq!(m.first(), Some(&ToolKind::Box));
        assert_eq!(matches("x-spline").first(), Some(&ToolKind::XSpline));
        assert!(matches("x-spline").contains(&ToolKind::XSplineClosed));
        assert!(matches("ellipse").contains(&ToolKind::EllipseRadii));
        assert!(matches("open/close").contains(&ToolKind::OpenClose));
        assert!(matches("circle").len() >= 2);
        assert!(matches("e").len() <= MAX_MATCHES);
    }

    #[test]
    fn several_words_all_must_match() {
        assert_eq!(matches("measure area"), vec![ToolKind::AreaMeasure]);
    }

    #[test]
    fn keys_come_from_the_balloon() {
        assert_eq!(mode_keys(ToolKind::Box, false), "b");
        assert_eq!(mode_keys(ToolKind::Arrow, false), "Shift-a");
        assert_eq!(mode_keys(ToolKind::AreaMeasure, false), "Ctrl-m");
    }
}
