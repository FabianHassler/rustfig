# Architecture

**Status:** 2026-10-16 (B12: fig2dev a program of its own; every export goes through it). Which xfig and fig2dev source file each module
corresponds to: [source-map.md](source-map.md).

RustFig is a Cargo workspace of layered crates. A crate may depend only on
lower layers; nothing below L6 may depend on egui, eframe, winit or rfd.
`cargo xtask layers` enforces this.

| Layer | Crate | Role |
|---|---|---|
| base | `figbase` | what fig2dev and RustFig share: points, X-splines, arcs, arrowheads, line widths; Fig font tables, metrics, glyph outlines (craft-fonts or fallback); colour tables |
| L0 | `rustfig-geom` | `figbase`'s geometry plus hit testing |
| L0 | `rustfig-fig2dev` | fig2dev: reader (`read.c`, Fig 1.3–3.2), `bound.c`, `gendev` (`convert`), option and language tables, every ported driver; the scene drivers (EPS, PDF, SVG, bitmaps) draw a `Scene` with its rasteriser and fill patterns |
| L1 | `rustfig-model` | document model: objects, styles, colours, ids, transforms, flattening |
| L1 | `rustfig-text` | `figbase`'s font tables and metrics plus text composition |
| L2 | `rustfig-format` | Fig 3.0–3.2 reader, Fig 3.2 writer, native JSON |
| L3 | `rustfig-render` | the canvas scene: a document → fig2dev's `Scene` (depth order, gray layers), drawn by fig2dev's rasteriser |
| L4 | `rustfig-engine` | session: commands, undo, selection, indicator, view, tools, guard, picture import; exports through `fig2dev::convert` |
| L5 | `rustfig-mcp` | MCP server over stdio |
| L6 | `rustfig-ui-egui` | egui app: canvas, panels, menus, dialogs, control channel |
| L7 | `rustfig` (app), `rustfig-cli`, `fig2dev` | binaries |

fig2dev is a program of its own that RustFig depends on (plan B12):
`rustfig-fig2dev` and the `fig2dev` app use no workspace crate but `figbase`
(checked by `cargo xtask layers`). Who uses whom (workspace crates only):

- `figbase` ← `rustfig-geom`, `rustfig-text`, `rustfig-model`, `rustfig-fig2dev`
- `rustfig-fig2dev` ← `apps/fig2dev`, `rustfig-render` (the `Scene` and its rasteriser), `rustfig-engine` (every export, old Fig files, pictures)
- `rustfig-geom` + `rustfig-model` ← `rustfig-format` ← `rustfig-engine`
- `rustfig-render` (canvas only) ← `rustfig-engine`, `rustfig-ui-egui`, `rustfig-cli`

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

Crash recovery (native, A6): `engine::recovery` keeps each instance's copy of
an unsaved drawing in the platform's recovery folder, next to an OS-locked
`ID.lock`; `ui-egui::autosave` writes it after a 3-second pause and from the
panic guard, and the next start offers drawings whose lock is free.

Canvas (A7): the scene is kept per drawing revision; on pan and zoom the last
raster is drawn moved and scaled to the new view at once, and the exact one is
rendered on a background thread when rendering is slow (native).

Tools (`crates/engine/src/tools`) turn pointer and key events (in Fig units)
into commands. While busy they expose an `Overlay` (preview objects, guides,
handles, the text being typed), which the UI renders with the same renderer.

UI-only commands (dialogs, quit) are in `crates/ui-egui/src/menus.rs`
(`UI_COMMANDS`).

## Rendering and export

The canvas: `rustfig_render::build_scene` flattens compounds, sorts leaves
in xfig's order (depth, deepest first; object type within a depth; gray
layers first) and turns each object into fig2dev's `Scene` primitives (paths
with solid or pattern fill and stroke, arrowheads, text runs, pictures),
which fig2dev's rasteriser (`rustfig_fig2dev::scene::raster`) draws. The
canvas caches the raster per (revision, view) and draws overlays separately.

Exports: the engine writes the document as Fig 3.2 text (`rustfig_format`),
reads it with fig2dev's reader and runs the language's driver through
`gendev::convert` (`crates/engine/src/fig2dev/mod.rs` `export`), passing
fig2dev's option letters as xfig's `u_print.c` does (magnification and bitmap
resolution → `-m`, margin → `-b`, background → `-g`, active layers →
`-D`/`-K`, `-F`). fig2dev's messages come back as warnings or errors. The
scene drivers (EPS, PDF, SVG, the bitmap languages) build a `Scene` from
fig2dev's objects with the same primitives the canvas uses
(`crates/fig2dev/src/scene/`), so what the canvas shows is what fig2dev
writes. Fig files before 3.0 open through fig2dev's reader
(`crates/engine/src/fig2dev/to_model.rs`).

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
| 2026-10-16 | B12: `figbase`, fig2dev at L0 and self-contained, render canvas-only, exports through `convert`. |
