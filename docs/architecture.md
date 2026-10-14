# Architecture

**Status:** 2026-10-14. Which xfig and fig2dev source file each module
corresponds to: [source-map.md](source-map.md).

RustFig is a Cargo workspace of layered crates. A crate may depend only on
lower layers; nothing below L6 may depend on egui, eframe, winit or rfd.
`cargo xtask layers` enforces this.

| Layer | Crate | Role |
|---|---|---|
| L0 | `rustfig-geom` | points, rects, affine transforms, X-splines, arcs, ellipses, arrowheads, hit testing |
| L1 | `rustfig-model` | document model: objects, styles, colours, ids, transforms, flattening |
| L1 | `rustfig-text` | Fig font tables, font selection, measurement, glyph outlines (craft-fonts or fallback) |
| L2 | `rustfig-format` | Fig 3.0–3.2 reader, Fig 3.2 writer, native JSON |
| L3 | `rustfig-render` | depth-sorted scene → tiny-skia raster, SVG, EPS, PDF, PNG; fill patterns |
| L4 | `rustfig-engine` | session: commands, undo, selection, indicator, view, tools, guard, picture import |
| L5 | `rustfig-mcp` | MCP server over stdio |
| L6 | `rustfig-ui-egui` | egui app: canvas, panels, menus, dialogs, control channel |
| L7 | `rustfig` (app), `rustfig-cli` | binaries |

## Everything is a command

User-visible behaviour lives in `crates/engine/src/cmd/*`. A command has an id,
label, menu path, shortcut, parameter docs, a `mutates` flag, `enabled` and
`run`. `Engine::execute` snapshots the document before a mutating command;
on error it restores the snapshot (commands are atomic), and on success it
pushes the snapshot onto the undo stack. Snapshots are cheap (A1): the
document's top-level objects are `Arc<Object>`, commands change one through
`Arc::make_mut` (or `Document::get_mut`), so the snapshot shares every object
the command did not touch; `history.rs` keeps each step only as the run of
the object list that differs from the state after it, and caps the history
at 200 steps and about 256 MB. After a command, `execute` re-measures the
texts of the objects it changed (A9); the hit-test box cache keeps each box
with its object, so only changed objects are recomputed.

Tools (`crates/engine/src/tools`) turn pointer and key events (in Fig units)
into commands. While busy they expose an `Overlay` (preview objects, guides,
handles, the text being typed), which the UI renders with the same renderer.

UI-only commands (dialogs, quit) are in `crates/ui-egui/src/menus.rs`
(`UI_COMMANDS`).

## Rendering

`Scene::build` flattens compounds, sorts leaves by depth (deepest first,
stable within a depth) and turns each object into primitives: paths with
fill (solid or pattern) and stroke (width, dash, cap, join), arrowheads,
text runs and images. One scene feeds every backend, so the canvas matches
the exports. The canvas caches the document raster per (revision, view) and
draws overlays separately.

## Robustness

No panics in shipped code: the workspace denies `unwrap`, `expect`, `panic!`
and friends outside tests. Input (files, pasted data, control and MCP
messages, command params) is validated and size-capped.
`rustfig_engine::guard` catches panics at every entry point (commands, tool
events, MCP calls, the UI frame) as a last line of defence. Parsers and
protocols are fuzzed with proptest.

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First version. |
