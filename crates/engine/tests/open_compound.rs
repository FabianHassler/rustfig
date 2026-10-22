#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Open compound (plan D1): `e_compound.c`. Members are edited in place with
//! every mode, the rest is hidden or gray, opens nest, closing returns up a
//! level, and saving, exporting and undo see one figure.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{Document, ObjectId, ObjectKind};
use rustfig_engine::{Button, Engine, SnapMode, ToolEvent, ToolKind};
use serde_json::{json, Value};

fn engine() -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e
}

fn rect(e: &mut Engine, x: i32, y: i32, w: i32, h: i32) -> ObjectId {
    e.execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h}))
        .unwrap();
    e.doc().objects.last().unwrap().id
}

/// A figure with a box, and a compound of a line and a circle next to it
/// (ids: 1 the box, 4 the compound).
fn figure() -> (Engine, ObjectId, ObjectId) {
    let mut e = engine();
    let outside = rect(&mut e, -3000, -3000, 600, 600);
    e.execute("shape.polyline", &json!({"points": [[0, 0], [1200, 0], [1200, 900]]}))
        .unwrap();
    e.execute("shape.circle", &json!({"cx": 3000, "cy": 0, "r": 300}))
        .unwrap();
    e.execute("edit.select_all", &json!({})).unwrap();
    // Group only the line and the circle.
    let ids: Vec<ObjectId> = e.doc().objects.iter().skip(1).map(|o| o.id).collect();
    e.set_selection(ids);
    e.execute("object.group", &json!({})).unwrap();
    let comp = e.doc().objects.last().unwrap().id;
    e.set_selection(Vec::new());
    (e, outside, comp)
}

fn json_of(e: &Engine) -> Value {
    serde_json::to_value(&*e.whole_doc()).unwrap()
}

fn click(e: &mut Engine, x: f64, y: f64, button: Button) {
    let pos = Point::new(x, y);
    e.tool_event(ToolEvent::Press {
        pos,
        button,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release { pos, button }).unwrap();
}

#[test]
fn members_become_the_drawing_and_close_restores_the_compound() {
    let (mut e, outside, comp) = figure();
    let whole = json_of(&e);
    e.execute("compound.open", &json!({"id": comp.0})).unwrap();
    assert_eq!(e.open_compounds(), 1);
    // The drawing is the compound's two members; the box is not reachable.
    assert_eq!(e.doc().objects.len(), 2);
    assert!(e.doc().get(outside).is_none());
    assert!(e
        .doc()
        .objects
        .iter()
        .all(|o| !matches!(o.kind, ObjectKind::Compound(_))));
    // The whole figure is unchanged for saving.
    assert_eq!(json_of(&e), whole);
    e.execute("compound.close", &json!({})).unwrap();
    assert_eq!(e.open_compounds(), 0);
    assert_eq!(json_of(&e), whole, "open and close change nothing");
    // The compound is back at its place, with its id.
    let c = e.doc().get(comp).unwrap();
    assert!(matches!(&c.kind, ObjectKind::Compound(c) if c.objects.len() == 2));
}

#[test]
fn editing_inside_keeps_the_compound_together() {
    let (mut e, outside, comp) = figure();
    e.execute("compound.open", &json!({"id": comp.0})).unwrap();
    let circle = e.doc().objects.last().unwrap().id;
    // Move a member by the Move tool, as the user does.
    e.set_tool(ToolKind::Move);
    e.tool_event(ToolEvent::Press {
        pos: Point::new(3300.0, 0.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Move {
        pos: Point::new(3300.0, 600.0),
    })
    .unwrap();
    e.tool_event(ToolEvent::Release {
        pos: Point::new(3300.0, 600.0),
        button: Button::Left,
    })
    .unwrap();
    // Draw a new member and edit another by command.
    rect(&mut e, 500, 2000, 300, 300);
    e.execute("object.set_attributes", &json!({"ids": [circle.0], "thickness": 5}))
        .unwrap();
    e.execute("compound.close", &json!({})).unwrap();
    let ObjectKind::Compound(c) = &e.doc().get(comp).unwrap().kind else {
        panic!("compound")
    };
    assert_eq!(c.objects.len(), 3, "the new box joined the compound");
    let ObjectKind::Ellipse(el) = &c.objects[1].kind else {
        panic!("ellipse")
    };
    assert_eq!(el.center, Point::new(3000.0, 600.0), "the circle moved inside");
    assert_eq!(el.style.thickness, 5);
    assert!(e.doc().get(outside).is_some());
    assert_eq!(e.doc().objects.len(), 2, "box and compound");
}

#[test]
fn opens_nest_and_close_goes_up_one_level() {
    let (mut e, _, comp) = figure();
    // Put the line and circle's group into a second level: group the
    // compound with a new box, then open both.
    let b = rect(&mut e, 5000, 0, 100, 100);
    e.set_selection(vec![comp, b]);
    e.execute("object.group", &json!({})).unwrap();
    let outer = e.doc().objects.last().unwrap().id;
    let whole = json_of(&e);
    e.execute("compound.open", &json!({"id": outer.0})).unwrap();
    let inner = e
        .doc()
        .objects
        .iter()
        .find(|o| matches!(o.kind, ObjectKind::Compound(_)))
        .unwrap()
        .id;
    e.execute("compound.open", &json!({"id": inner.0, "keep_visible": true}))
        .unwrap();
    assert_eq!(e.open_compounds(), 2);
    assert_eq!(e.doc().objects.len(), 2);
    assert_eq!(json_of(&e), whole);
    e.execute("compound.close", &json!({})).unwrap();
    assert_eq!(e.open_compounds(), 1);
    assert!(e.doc().get(inner).is_some(), "back in the outer compound");
    assert_eq!(e.tool(), ToolKind::OpenCompound, "xfig selects the mode again");
    e.execute("compound.open", &json!({"id": inner.0})).unwrap();
    e.execute("compound.close_all", &json!({})).unwrap();
    assert_eq!(e.open_compounds(), 0);
    assert_eq!(json_of(&e), whole);
    assert!(
        e.execute("compound.close", &json!({})).is_err(),
        "disabled when nothing is open"
    );
}

#[test]
fn a_compound_emptied_inside_is_removed_on_close() {
    let (mut e, outside, comp) = figure();
    e.execute("compound.open", &json!({"id": comp.0})).unwrap();
    e.execute("edit.select_all", &json!({})).unwrap();
    e.execute("edit.delete", &json!({})).unwrap();
    let r = e.execute("compound.close", &json!({})).unwrap();
    assert_eq!(r["removed_empty"], json!(true));
    assert!(e.doc().get(comp).is_none());
    assert_eq!(e.doc().objects.len(), 1);
    assert!(e.doc().get(outside).is_some());
}

#[test]
fn undo_goes_through_open_and_close() {
    let (mut e, _, comp) = figure();
    let whole = json_of(&e);
    e.execute("compound.open", &json!({"id": comp.0})).unwrap();
    rect(&mut e, 0, 5000, 100, 100);
    let edited = json_of(&e);
    assert_ne!(edited, whole);
    e.execute("compound.close", &json!({})).unwrap();
    assert_eq!(json_of(&e), edited);
    // Undo the close: inside again, with the new member.
    assert!(e.undo());
    assert_eq!(e.open_compounds(), 1);
    assert_eq!(e.doc().objects.len(), 3);
    assert_eq!(json_of(&e), edited);
    // Undo the new box, then the open.
    assert!(e.undo());
    assert_eq!(e.doc().objects.len(), 2);
    assert!(e.undo());
    assert_eq!(e.open_compounds(), 0);
    assert_eq!(json_of(&e), whole);
    // And forward again.
    assert!(e.redo());
    assert_eq!(e.open_compounds(), 1);
    assert!(e.redo());
    assert!(e.redo());
    assert_eq!(e.open_compounds(), 0);
    assert_eq!(json_of(&e), edited);
}

#[test]
fn saving_and_exporting_see_the_whole_figure() {
    let (mut e, _, comp) = figure();
    let data =
        |e: &mut Engine, fmt: &str| e.execute("file.save_data", &json!({"format": fmt})).unwrap()["data"].clone();
    let fig = data(&mut e, "fig");
    let native = data(&mut e, "rustfig");
    let svg = e.execute("file.export_data", &json!({"format": "svg"})).unwrap();
    e.execute("compound.open", &json!({"id": comp.0})).unwrap();
    assert_eq!(data(&mut e, "fig"), fig);
    assert_eq!(data(&mut e, "rustfig"), native);
    assert_eq!(e.execute("file.export_data", &json!({"format": "svg"})).unwrap(), svg);
    // Reading the saved file gives the closed figure.
    let mut e2 = engine();
    e2.execute("file.open_data", &json!({"data": fig})).unwrap();
    assert_eq!(e2.open_compounds(), 0);
    assert_eq!(e2.doc().objects.len(), 2);
    // Depths and info count the rest, too.
    let info = e.execute("doc.info", &json!({})).unwrap();
    assert_eq!(info["open_compounds"], json!(1));
    assert_eq!(info["depths"], json!(e.whole_doc().depths()));
    // A new figure leaves nothing open.
    e.execute("file.new", &json!({})).unwrap();
    assert_eq!(e.open_compounds(), 0);
}

#[test]
fn delete_all_closes_first() {
    let (mut e, _, comp) = figure();
    e.execute("compound.open", &json!({"id": comp.0})).unwrap();
    e.execute("edit.delete_all", &json!({})).unwrap();
    assert_eq!(e.open_compounds(), 0);
    assert!(e.doc().objects.is_empty());
    assert!(e.undo());
    assert_eq!(e.open_compounds(), 1);
}

#[test]
fn the_open_compound_mode_and_its_buttons() {
    let (mut e, outside, _) = figure();
    e.set_tool(ToolKind::OpenCompound);
    let h = e.mouse_hints();
    assert_eq!((h.left, h.middle, h.right), ("open compound", "open, keep visible", ""));
    // Markers only on compounds.
    assert_eq!(e.mode_markers().len(), 4);
    // The box is not a compound: nothing happens.
    click(&mut e, -3000.0, -3000.0, Button::Left);
    assert_eq!(e.open_compounds(), 0);
    assert!(e.doc().get(outside).is_some());
    // Right does nothing; left opens.
    click(&mut e, 1200.0, 400.0, Button::Right);
    assert_eq!(e.open_compounds(), 0);
    click(&mut e, 1200.0, 400.0, Button::Left);
    assert_eq!(e.open_compounds(), 1);
    assert!(!e.doc().open[0].keep_visible);
    // Inside, the mode works on the members: the markers are gone (no
    // compounds), and the Move tool reaches a member.
    assert!(e.mode_markers().is_empty());
    e.execute("compound.close", &json!({})).unwrap();
    // The middle button keeps the rest visible.
    e.set_tool(ToolKind::OpenCompound);
    click(&mut e, 1200.0, 400.0, Button::Middle);
    assert!(e.doc().open[0].keep_visible);
}

#[test]
fn the_rest_is_gray_only_when_kept_visible() {
    let (mut e, _, comp) = figure();
    let opts = rustfig_engine::render::SceneOptions::default();
    let whole = rustfig_engine::render::build_scene(&e.whole_doc(), &opts).prims.len();
    e.execute("compound.open", &json!({"id": comp.0})).unwrap();
    let alone = rustfig_engine::render::build_scene(e.doc(), &opts);
    assert!(alone.prims.len() < whole, "the rest is not drawn");
    e.execute("compound.close", &json!({})).unwrap();
    e.execute("compound.open", &json!({"id": comp.0, "keep_visible": true}))
        .unwrap();
    let kept = rustfig_engine::render::build_scene(e.doc(), &opts);
    assert_eq!(kept.prims.len(), whole, "the rest is drawn behind");
    // The box's stroke (drawn first) is gray, the members' are not.
    let strokes: Vec<_> = kept
        .prims
        .iter()
        .filter_map(|p| match p {
            rustfig_engine::render::Prim::Path { stroke: Some(s), .. } => Some(s.color),
            _ => None,
        })
        .collect();
    let first = strokes.first().unwrap();
    assert!(
        first.r == first.g && first.g == first.b && first.r > 100,
        "gray backdrop: {first:?}"
    );
    let last = strokes.last().unwrap();
    assert_eq!((last.r, last.g, last.b), (0, 0, 0), "members in their colours");
}

#[test]
fn bad_requests_fail_and_change_nothing() {
    let (mut e, outside, comp) = figure();
    let before: Document = e.doc().clone();
    for (id, p) in [
        ("compound.open", json!({})),
        ("compound.open", json!({"id": 99999})),
        ("compound.open", json!({"id": outside.0})),
        ("compound.open", json!({"id": "x"})),
        ("compound.open", json!({"id": comp.0, "keep_visible": "yes"})),
        ("compound.close", json!({})),
        ("compound.close_all", json!({})),
    ] {
        assert!(e.execute(id, &p).is_err(), "{id} {p}");
        assert_eq!(e.doc(), &before);
    }
    // An action in progress refuses.
    e.set_tool(ToolKind::Polyline);
    click(&mut e, 0.0, 5000.0, Button::Left);
    assert!(e.tool_busy());
    assert!(e.execute("compound.open", &json!({"id": comp.0})).is_err());
    assert_eq!(e.open_compounds(), 0);
    // The same object cannot be opened twice.
    e.set_tool(ToolKind::None);
    e.execute("compound.open", &json!({"id": comp.0})).unwrap();
    assert!(e.execute("compound.open", &json!({"id": comp.0})).is_err());
}

#[test]
fn deep_nesting_is_bounded() {
    let mut e = engine();
    rect(&mut e, 0, 0, 100, 100);
    // Group the box with itself n times: 300 compounds inside each other.
    for _ in 0..300 {
        let id = e.doc().objects[0].id.0;
        let other = rect(&mut e, 0, 0, 10, 10);
        e.set_selection(vec![ObjectId(id), other]);
        e.execute("object.group", &json!({})).unwrap();
    }
    let mut opened = 0;
    while let Some(c) = e
        .doc()
        .objects
        .iter()
        .find(|o| matches!(o.kind, ObjectKind::Compound(_)))
        .map(|o| o.id)
    {
        if e.execute("compound.open", &json!({"id": c.0})).is_err() {
            break;
        }
        opened += 1;
    }
    assert!(opened >= 256, "{opened}");
    e.execute("compound.close_all", &json!({})).unwrap();
    assert_eq!(e.open_compounds(), 0);
}

/// xfig's "Save All" / "Save Part" (`do_save`, `w_file.c`): the engine's
/// default is the whole figure; `part: true` writes only the open
/// compound's members.
#[test]
fn saving_part_writes_only_the_visible_members() {
    let (mut e, _, comp) = figure();
    e.execute("compound.open", &json!({"id": comp.0})).unwrap();
    let data = |e: &mut Engine, p: Value| {
        let d = e.execute("file.save_data", &p).unwrap()["data"].clone();
        rustfig_engine::format::read_any(d.as_str().unwrap().as_bytes()).unwrap()
    };
    let whole = data(&mut e, json!({"format": "fig"}));
    assert_eq!(whole.objects.len(), 2, "the box and the compound");
    let part = data(&mut e, json!({"format": "fig", "part": true}));
    assert_eq!(part.objects.len(), 2, "the line and the circle");
    assert!(part.objects.iter().all(|o| !matches!(o.kind, ObjectKind::Compound(_))));
    // A file save takes the same choice, and the figure stays open.
    let dir = std::env::temp_dir().join(format!("rustfig-save-part-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("part.fig").to_string_lossy().into_owned();
    e.execute("file.save", &json!({"path": path, "part": true})).unwrap();
    let text = std::fs::read(&path).unwrap();
    let saved = rustfig_engine::format::read_any(&text).unwrap();
    assert!(saved.objects.iter().all(|o| !matches!(o.kind, ObjectKind::Compound(_))));
    assert_eq!(e.open_compounds(), 1);
    e.execute("file.save", &json!({"path": path})).unwrap();
    let text = std::fs::read(&path).unwrap();
    let saved = rustfig_engine::format::read_any(&text).unwrap();
    assert!(saved.objects.iter().any(|o| matches!(o.kind, ObjectKind::Compound(_))));
    // A bad value is refused.
    assert!(e.execute("file.save_data", &json!({"part": "yes"})).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
