# Architecture

**Status:** 2026-10-21 (DD9: sweep of stale statements); 2026-10-21 (DD7: exports from the bridge's objects); 2026-10-16 (B12: fig2dev a program of its own; every export goes through it). Which xfig and fig2dev source file each module
corresponds to: [source-map.md](source-map.md).

RustFig is a Cargo workspace of layered crates. A crate may depend only on
lower layers; nothing below L6 may depend on egui, eframe, winit or rfd.
`cargo xtask layers` enforces this.

| Layer | Crate | Role |
|---|---|---|
| ext | `figbase` (FabianHassler/fig2dev) | what fig2dev and RustFig share: points, X-splines, arcs, arrowheads, line widths; Fig font tables, metrics, glyph outlines (system fonts, TeX Gyre, or the built-in fallback); colour tables |
| L0 | `rustfig-geom` | `figbase`'s geometry plus hit testing |
| ext | `fig2dev` (FabianHassler/fig2dev) | fig2dev: reader (`read.c`, Fig 1.3–3.2), `bound.c`, `gendev` (`convert`), option and language tables, every ported driver; the scene drivers (EPS, PDF, SVG, bitmaps) draw a `Scene` with its rasteriser and fill patterns |
| L1 | `rustfig-model` | document model: objects, styles, colours, ids, transforms, flattening |
| L1 | `rustfig-text` | `figbase`'s font tables and metrics plus text composition |
| L2 | `rustfig-format` | Fig 1.3–3.2 reader (fig2dev's, through the bridge), Fig 3.2 writer, native JSON |
| L3 | `rustfig-render` | the canvas scene: a document → fig2dev's `Scene` (depth order, gray layers), drawn by fig2dev's rasteriser |
| L4 | `rustfig-engine` | session: commands, undo, selection, indicator, view, tools, guard, picture import; exports through `fig2dev::convert` |
| L5 | `rustfig-mcp` | MCP server over stdio |
| L5 | `rustfig-os` | OS calls: pick a colour from the screen (E4b) and the native print dialog for a PDF (B5b): the only crate with `unsafe` (`deny` plus `allow` on the few OS-call functions: macOS NSColorSampler and PDFKit print operation, Windows GetPixel and `PrintDlgEx`); safe API, no egui |
| L6 | `rustfig-ui-egui` | egui app: canvas, panels, menus, dialogs, control channel |
| L7 | `rustfig` (app), `rustfig-cli`, `fig2dev` | binaries |

fig2dev is a program of its own that RustFig depends on (plan B12):
`figbase`, `fig2dev` and the `fig2dev` app live in their own repository,
<https://github.com/FabianHassler/fig2dev>, and RustFig bundles its command line: `rustfig fig2dev ARGS`, `rustfig-cli fig2dev ARGS`, or the `rustfig` binary started as `fig2dev` call `fig2dev::cli::run` before the GUI parses anything (`engine::fig2dev::run_cli`, plan DD10). RustFig takes the two
crates as git dependencies (a `[patch]` points at a local clone `../fig2dev`
while it is not published). They use no RustFig crate. Who uses whom (workspace crates only):

- `figbase` ← `rustfig-geom`, `rustfig-text`, `rustfig-model`, `fig2dev`
- `fig2dev` ← `rustfig-render` (the `Scene` and its rasteriser), `rustfig-engine` (every export, old Fig files, pictures)
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
which fig2dev's rasteriser (`fig2dev::scene::raster`) draws. The
canvas caches the raster per (revision, view) and draws overlays separately.

Exports: the engine converts the document to fig2dev's objects and context
(`rustfig_format::bridge::export_objects`: what fig2dev's reader returns for
the Fig 3.2 text the document would be saved as, repairs included, built with
fig2dev's own repair functions) and runs the language's driver through
`gendev::convert` (`crates/engine/src/fig2dev/mod.rs` `export`). Where the
reader would print a message or refuse something (`export_objects` is `None`),
`export` writes the Fig text and reads it, as xfig does (`export_via_text`, the
reference of the identity tests), so messages and errors are fig2dev's. It passes
fig2dev's option letters as xfig's `u_print.c` does (magnification and bitmap
resolution → `-m`, margin → `-b`, background → `-g`, active layers →
`-D`/`-K`, `-F`). fig2dev's messages come back as warnings or errors. The
scene drivers (EPS, PDF, SVG, the bitmap languages) build a `Scene` from
fig2dev's objects with the same primitives the canvas uses
(fig2dev repo, `crates/fig2dev/src/scene/`), so what the canvas shows is what fig2dev
writes. Every Fig file (1.3–3.2) opens through fig2dev's reader (`Ctx::editing`), converted by the
bridge (`crates/format/src/bridge/`, L2: `to_model` fig2dev objects → document,
`from_model` document → fig2dev `Ctx` + objects, which equal what fig2dev's reader
returns for `write_fig`'s text; `export_objects` adds the reader's repairs for the exports). `format`
depends on the `fig2dev` crate for this (it sits below every layer).

## Copy as picture (E14)

`edit.copy_picture` makes the bytes with fig2dev's drivers (`picture.rs`: macOS PDF + PNG, Linux SVG + PNG, Windows EMF/SVG + PNG) and hands them to a `PictureClipboard` the front end installs (`Engine::set_picture_clipboard`). `rustfig-ui-egui` installs `clipboard::SystemPictureClipboard`, built on the `clipboard-rs` crate (MIT OR Apache-2.0, native only; one clipboard write with several custom types). Without a sink (wasm, MCP, CLI, tests) the command reports that it is not available; tests use a fake.

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
| 2026-10-21 | DD9: removed the `engine::fig2dev::to_model` re-export from the text. |
| 2026-10-21 | DD7: exports from the bridge's objects (`export_objects`), text path as reference and fallback. |
| 2026-10-22 | E14: Copy as picture; `clipboard-rs` in `rustfig-ui-egui`. |
