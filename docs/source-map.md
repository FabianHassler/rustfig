# Source map: xfig and fig2dev → RustFig

**Paths written `fig2dev:crates/…` are in the fig2dev repository (<https://github.com/FabianHassler/fig2dev>), where figbase, the fig2dev library and the `fig2dev` program moved (B13).**

**Status:** 2026-10-23 (S4-7: `w_mousefun.c` complete). 2026-10-22 (D5: `e_measure.c` ported); 2026-10-22 (D4: `e_tangent.c` ported with the curve search of `u_smartsearch.c`/`u_geom.c`); 2026-10-22 (D14: `engine/links.rs` generalised); 2026-10-22 (D7: `u_elastic.c`/`u_geom.c` constraints in `engine/anglegeom.rs`; D8: smart links, `u_list.c`'s linked move in `engine/links.rs`, the Smart Links button; D3: `e_chop.c` ported, `geom/intersect.rs` from `w_intersect.c`); 2026-10-21 (S4: the xfig file statuses corrected from the function-level audit, [`source-audit.md`](source-audit.md); DD9: sweep; DD7: `read.c`'s repairs as functions used by `format/bridge/export`); 2026-10-20 (DD3: `f_read.c` read through fig2dev's reader); 2026-10-18 (B9: emf, tk, ptk, tkpattern; B12: shared code in `figbase`, the scene writers in fig2dev's crate, every export through fig2dev; B3: pict2e; B1: tikz; B3: pstricks). First version, after the decision to port from xfig's
sources (`LICENSE-xfig`). Nothing has been ported line by line yet: every
"own" row was written from the manual and the format description and must be
checked against the C file named in it.

Function by function: [`source-audit.md`](source-audit.md) (S4) lists every xfig function with its status; the file rows below summarise it (a status marked "(S4)" comes from there). fig2dev's functions were audited in the fig2dev repository.

Legend: **own** RustFig has its own implementation (review against the source
pending) · **partial** part of the file's behaviour exists · **missing** ·
**n/a** X11/Xt plumbing that RustFig replaces with its own design (egui, the
renderer, native dialogs) · **modern** replaced on purpose by a modern
alternative.

Line counts are the C files in `old/xfig-3.2.9a/src/` (≈100 k lines; 44 k of
them are `w_*` widget code for the X toolkit, which can't be ported as is) and
`old/fig2dev-3.2.9a/fig2dev/` (≈50 k lines).

## Findings so far

1. **Draw order within a depth differed** (fixed in S1). xfig keeps one list per object type
   and, for each depth from back to front, draws arcs, compounds (recursively,
   same order inside), ellipses, lines, splines, texts (`u_redraw.c`
   `redisplay_objects`); `f_save.c` writes the same order. RustFig draws in
   file order within a depth. Same picture for files xfig wrote; different for
   objects created in RustFig. Fix: a sort key in the scene builder (S1; now `rustfig_render::build_scene`).
2. **Gray depths are drawn behind everything** (fixed in S1). xfig draws all inactive
   layers in gray first, then the active ones (`u_redraw.c`); RustFig keeps
   them at their own depth (S1).
3. **Mouse indicator layout** (fixed in S2, with xfig's wording and two
   behaviours the wording revealed: ellipses finish with either button,
   and in text mode middle finishes and right cancels). xfig puts the left label left of the mouse,
   the middle label above it and the right label right of it
   (`w_mousefun.c`, manual image `mouse-function-indicator.png`); W2 used
   three columns. Port the layout (S2).
4. **No conflict in the data model.** xfig's per-type lists (`object.h`
   `F_compound`) are reproduced by the ordering above; RustFig's single list
   with ids is a superset. xfig's undo is one level per action type
   (`u_undo.c`); RustFig's snapshot undo is multi-level, a superset.
5. **Startup options and resources matter to users.** `main.c` / `resources.c`
   / `xfigmain.c` define the command-line options and X resources
   (`-metric`, `-specialtext`, `-latexfonts`, `-exportLanguage`,
   `-startgridmode`, arrow defaults, …) that set xfig's defaults; `f_util.c`
   reads `~/.xfigrc`. RustFig has no settings persistence yet (E1); porting
   the option set gives users their familiar defaults (S3).
6. **Seen in the real xfig** (3.2.9a under XQuartz with the user's
   options, captured with `xwd`): the depth panel shows no object counts
   (RustFig's are an addition); a "Snap Mode" box sits above the depth
   panel (W4); with no mode chosen the attribute panel shows only Zoom and
   Grid Mode (W5); rulers in the chosen units with the units box in the
   corner (W6); the mode panel labels some icons (Picture, Scale, Align,
   Move, Copy, Delete, Update, Edit, Rotate).
7. **Exports can be ported from fig2dev.** `gentikz.c`, `genpstex.c`,
   `genpict2e.c`, `genpstricks.c`, `genlatex.c`, `genps.c` cover phase B
   directly; `trans_spline.c` and `bound.c` are their shared helpers.

## xfig: drawing modes (`d_*`, 3.6 k lines)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `d_arc.c` | 359 | arc by three points or by centre | `engine/tools` (Arc) | ported (S4) | — |
| `d_arcbox.c` | 132 | rounded box | `engine/tools` (ArcBox) | ported (S4) | — |
| `d_box.c` | 133 | box | `engine/tools` (Box) | ported (S4) | — |
| `d_ellipse.c` | 345 | circles/ellipses by radius or diameter | `engine/tools` | ported (S4) | — |
| `d_line.c` | 318 | polylines, polygons, freehand, dimension lines | `engine/tools` | partial: dimension lines ported (D10, `dimline.rs`); latex/manhattan/mountain constraints ported (D7) | D10 |
| `d_picobj.c` | 136 | picture box | `engine/tools` (Picture) | ported (S4) | — |
| `d_regpoly.c` | 149 | regular polygon | `engine/tools` | ported (S4) | — |
| `d_spline.c` | 195 | splines | `engine/tools` | ported (S4) | — |
| `d_subspline.c` | 179 | spline helpers | `fig2dev:crates/figbase/src/geom` (via `geom`) | replaced: the overlay evaluates the whole curve (S4) | — |
| `d_text.c` | 1477 | text entry, cursor, compose keys, paste | `engine/tools` (Text) | partial: Up/Down, caret by mouse, Ctrl-key editing, paste into text (S4) | E10, F3 |

## xfig: editing modes (`e_*`, 17 k lines)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `e_addpt.c` | 368 | add point | `engine/tools` | ported (S4) | — |
| `e_align.c` | 962 | align compound / canvas, distribute | `cmd/arrange`, tools | partial: align to canvas ported (D10); distribute choices in the mode (S4) | D10, W10-33 |
| `e_arrow.c` | 293 | add/remove arrowheads | tools, `cmd/object` | ported (S4) | — |
| `e_break.c` | 92 | break compound (and tag) | tools | ported (S4) | — |
| `e_chop.c` | 890 | chop objects at intersections | `cmd/chop.rs`, `tools/chop.rs`, extended by `cmd/outline.rs`, `cmd/chop_ext.rs` (D15) | ported (S4) | D3, D15 |
| `e_compound.c` | 294 | open/close compound | `model::Document::open_compound`, `cmd/compound.rs`, `tools/compound.rs`, `panels::close_compound_popup` | ported (dimension-line recomputation on close: D10) | D1 |
| `e_convert.c` | 420 | line↔spline, box↔arc-box | `cmd/object` | partial: box ↔ arc-box (S4) | S4-5 |
| `e_copy.c` | 185 | copy, array copy, copy to cut buffer | tools | ported: array copy (D10) | D10 |
| `e_delete.c` | 307 | delete object/region, cut buffer | tools | ported (S4) | — |
| `e_deletept.c` | 214 | delete point | tools | ported (S4) | — |
| `e_edit.c` | 5996 | the Edit panel for every object type, figure comments, spline point shapes | `ui-egui/dialogs` | partial: compound and picture rows, point table, shape factor on the canvas, live panel (S4) | W10-15, W10-16 (done), W10-19, S4-6 |
| `e_flip.c` | 452 | flip, copy & flip, anchor | tools, `cmd/transform` | ported: anchor (D10) | D10 |
| `e_glue.c` | 521 | tag and glue compounds | tools | ported (S4) | — |
| `e_joinsplit.c` | 714 | join/split lines and splines | `cmd/joinsplit.rs`, `tools/compound.rs` | ported | D2 |
| `e_measure.c` | 514 | measure angle, length, area | `cmd/measure.rs`, `tools/measure.rs` | ported (S4) | D5 |
| `e_move.c` | 135 | move (constrained) | tools | ported (S4) | — |
| `e_movept.c` | 845 | move point (constrained, linked) | tools | ported; xfig does not link here, a box corner resizes the attached ends in RustFig (D14) | D14 |
| `e_placelib.c` | 337 | place library object | — | missing (11 functions, S4) | D6 |
| `e_rotate.c` | 575 | rotate, copy & rotate, anchor, copies count | tools | ported: centre, number of copies (D10); the Num Copies button missing | D10 |
| `e_scale.c` | 1444 | scale box/edge/centre | tools | ported: edge scaling in one dimension, dimension lines (D10) | D10 |
| `e_tangent.c` | 159 | tangent/normal lines | `cmd/tangent.rs`, `tools/tangent.rs`; curve search from `u_smartsearch.c`, `u_geom.c` | ported (S4) | D4 |
| `e_update.c` | 827 | update object / settings, update mask | tools, `indicator.take_from` | ported: update flags (`UpdateFlag`, `Indicator::update_object`, `take_from`) | W10-1 |

## xfig: files (`f_*`, 9.9 k lines)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `f_read.c` | 2034 | Fig 3.x reader (all quirks) | fig2dev's `read.c` through `format/bridge/to_model` (`format/fig_read`, 83 lines; DD3) | via fig2dev | DD3 ✅ |
| `f_readold.c` | 537 | Fig 1.x/2.x reader | fig2dev's `read/read1_3` and `read.c` through the bridge (`format/fig_read`) | via fig2dev | G1, DD3 ✅ |
| `f_save.c` | 665 | Fig 3.2 writer (type order) | `format/fig_write` | ported: type order, header, arrows, comments; `.bak` on save (`write_bak`) | — |
| `f_load.c` | 384 | load, merge | `cmd/file` | ported (S4) | — |
| `f_util.c` | 1291 | directories, image colours, `~/.xfigrc` | — | partial: `~/.xfigrc` replaced by TOML (E1); `xfig -update`, dimension-line parts missing; image colormap code not needed (S4) | S3, D10 |
| `f_picobj.c` | 970 | picture loading and caching | `engine/import` | replaced by fig2dev `readpics`; relative picture names rewritten on Save As (S4-3 done) | S4-3 |
| `f_readeps.c` | 343 | EPS/PDF pictures (bounding box, ghostscript preview) | `fig2dev:crates/fig2dev/src/scene/eps.rs` (EPS pictures embedded in EPS output), `util/{pdfpic,pdfps,gs}.rs` (PDF pictures through hayro; ghostscript for EPS in PDF/bitmaps, optional); canvas: `engine/src/import.rs` renders PDF pages and EPS through gs or the preview | ported (B15) | B15 |
| `f_readgif.c`, `f_readjpg.c`, `f_readpng.c`, `f_readppm.c`, `f_readtif.c` | 1.6 k | bitmap pictures | `image` crate | modern | — |
| `f_readpcx.c`, `f_readxbm.c`, `f_readxpm.c` | 790 | PCX, XBM, XPM pictures | fig2dev `util/readpics` (DD5) | replaced (S4) | — |
| `f_wrpcx.c`, `f_wrpng.c` | 317 | bitmap writers (for export) | `fig2dev:crates/fig2dev/src/scene/{raster,bitmaps}.rs` (PNG and fig2dev's bitmap languages) | replaced: fig2dev bitmaps (B6 ✅) (S4) | — |
| `f_neuclrtab.c` | 596 | colour quantisation for X visuals | — | n/a | — |

## xfig: utilities (`u_*`, 19 k lines)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `u_draw.c` | 2237 | drawing every object type, arrowheads, dash patterns | `render` (canvas scene), `fig2dev:crates/fig2dev/src/scene/` (primitives), `fig2dev:crates/figbase/src/geom/arrow.rs` | replaced by the renderer (fig2dev scene); arrowheads ported from fig2dev `bound.c` (D9) (S4) | — |
| `u_draw_spline.c` | 384 | X-spline evaluation | `fig2dev:crates/figbase/src/geom/xspline.rs` (from the paper) | own | review (step size) |
| `u_redraw.c` | 731 | redraw order, gray layers | `rustfig_render::build_scene` | ported: draw order, gray layers first | — |
| `u_bound.c` | 653 | bounding boxes | `fig2dev:crates/fig2dev/src/bound.rs` through `crates/format/src/bridge/bounds.rs` (DD2); `model` keeps only `Object::approx_bbox` (hit-test prefilter) and `geometry_bbox` | ported via fig2dev's `bound.c` | — |
| `u_geom.c` | 1072 | normals, slopes, latex/manhattan endpoints, closeness | `geom` | partial: lengths, areas and angles ported to `cmd/measure.rs`; `get_slope`, `latex_endpoint` and the slope tables ported to `engine/anglegeom.rs` (D7) | D5 |
| `u_quartic.c` | 298 | quartic solver (intersections) | `snap.rs` (flattened outlines) | modern | — |
| `u_elastic.c` | 1004 | rubber-band feedback while drawing | `Engine::overlay` | replaced by `Engine::overlay`; latex/manhattan/mountain constraints ported to `engine/anglegeom.rs` (D7); smart-link rubber bands done (D8; every follower, box-corner and scale previews D14) | |
| `u_drag.c` | 861 | dragging objects | tools, overlay | ported, with array placement (D10) | D10 |
| `u_markers.c` | 462 | object markers, tag highlight | `engine/tools/markers.rs` (masks, markers), `ui-egui/canvas` | ported (W12); highlight of the found object missing (S4) | S4-1 |
| `u_list.c` | 1358 | object lists, depth bookkeeping, linked move | `model::Document`, `engine/links.rs` | replaced by `model::Document`; smart-link bookkeeping (`get_links`, `adjust_links`) ported to `engine/links.rs` (S4); extended to every shape and operation, copy with links (D14) | — |
| `u_create.c`, `u_free.c` | 1.4 k | allocation | — | n/a | — |
| `u_translate.c`, `u_scale.c` | 314 | translate/scale objects | `model` transforms | own | — |
| `u_convert.c` | 259 | line↔spline conversion | `cmd/object` | replaced: UTF-8 strings, raw Fig bytes (S4) | — |
| `u_colors.c` | 306 | colour tables | `figbase/color` (via `model`) | own | review |
| `u_fonts.c` | 1264 | PostScript/LaTeX font tables | `text` | ported: font tables, metrics, Symbol/Dingbats maps; X font loading replaced (S4) | — |
| `u_search.c` | 1029 | picking objects, points, segments | `Engine::hit_test`, tools | ported; Shift-click cycling and highlight missing (S4) | S4-1 |
| `u_smartsearch.c` | 507 | smart search (Shift-click cycling, object highlight) | — | missing (20 functions, S4) | S4-1 |
| `u_undo.c` | 962 | undo (one level) | `engine/history` (multi-level) | replaced: multi-level snapshot undo (S4) | — |
| `u_error.c` | 136 | emergency save on crash | `engine/guard` | replaced: `engine::guard` and recovery files (A6) (S4) | — |
| `u_pan.c` | 89 | pan steps | `rulers::pan_step` | ported | — |
| `u_print.c` | 923 | print and export through fig2dev (passes `-F`, true point sizes, for PS/EPS/PDF) | `cmd/file` export | export replaced (fig2dev in process); printing: `print.rs` builds the same fig2dev options, PDF to the OS print system (B5) | B5 |
| `u_ghostscript.c` | 1218 | EPS/PDF preview through ghostscript | `fig2dev:crates/fig2dev/src/util/{pdfpic,pdfps,gs}.rs` | replaced: fig2dev `util/{pdfpic,pdfps,gs}.rs` (B15); library callbacks not needed (S4) | — |
| `u_spawn.c` | 587 | run external programs | — | replaced: no spawned exports; image editor and help viewer through the OS (S4) | W10-16, W10-2 |

## xfig: window (`w_*`, 44 k lines; rewritten in egui)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `w_cmdpanel.c` | 1860 | main menus | `menus::XFIG_MENUS` (classic layout) | ported: menus; partial: paste with the mouse, character map, global settings rows (S4) | S4-2, E5, W10-23 |
| `w_modepanel.c` | 905 | mode panel, mode keys, per-mode attribute masks | `panels::mode_panel`, `ToolKind::shortcut`, `attributes_for` | ported: buttons, keys, masks; Open compound and Join/Split work (D1, D2); the Chop, Tangent, Measure and Library modes are greyed or missing (S4) | D1–D6 |
| `w_icons.c` | 3270 | mode and attribute icon bitmaps | `ui-egui/xfig_icons.rs` (verbatim), used by `panels::mode_panel` and `attributes` | ported: mode icons and the attribute pictures (line, join, cap, arrow mode and type, arc type, justification, grid, positioning, alignment) | — |
| `w_indpanel.c` | 4865 | attribute (indicator) panel, choice panels | `ui-egui/attributes` | ported: buttons, order, per-mode masks, choice panels, Update Control, Smart Links; Angle Geom; missing: copy counts, dimension line, tangent length, colour editor, Arrow Size panel (function by function in `source-audit.md`) | D10, D4, E4, W10-31 |
| `w_mousefun.c` | 432 | mouse-function indicator | `panels::mouse_indicator`, `Engine::mouse_hints` | ported, with the Shift/Ctrl/Ctrl+Shift variants (S4-7, 2026-10-23) | S4-7 |
| `w_msgpanel.c` | 881 | message panel, lengths while drawing | `panels::message_panel` | partial: lengths while drawing (D10, `tools/lengths.rs`); measure results (S4) | D10, D5 |
| `w_layers.c` | 903 | depth panel | `panels::depth_panel` | ported (S4) | — |
| `w_rulers.c` | 1479 | rulers and units box | `ui-egui/rulers`, `view.set_scale` | ported: tick tables, labels, marker, ruler buttons, units box, Unit menu | — |
| `w_snap.c`, `w_intersect.c` | 2.8 k | Snap menu (object snaps); intersections for Chop | `engine/snap.rs`, `snap.*` commands, `geom/intersect.rs` | ported (curves on flattened outlines instead of the quartic solver) | — |
| `w_keyboard.c` | 518 | type coordinates for the next point | — | partial: `tool.point` and `coords.rs` ported (D10); the entry box and history missing | D10 |
| `w_zoom.c` | 202 | zoom panel | `cmd/view`, `tools/ctrl.rs` | zoom box (E3) done; unzoom is 100 % (previous-view semantics D11) | D11 |
| `w_grid.c` | 238 | grid drawing | canvas | ported (S4) | — |
| `w_canvas.c` | 1218 | canvas input, key bindings, compose key | `ui-egui/canvas`, `menus::pan_keys`, `menus::Compose`, `rustfig_text::compose` (CompKeyDB) | own; partial: Ctrl+click zoom, shape factor on the canvas, crosshair (S4) | E3, S4-6, E9 |
| `w_cursor.c` | 144 | cursors per mode | canvas | partial: one cursor (S4) | S4-4 |
| `w_color.c` | 2592 | colour panel and editor | indicator combo | replaced: the Colours window (`color_dialog.rs`) and `color.*` commands (E4, W10-36); mix memory and lock not built | E4 |
| `w_export.c` | 2054 | export panel | `dialogs::export` (language menu in xfig's order), default suffixes in `fig2dev:crates/fig2dev/src/languages.rs` | ported: Export panel options (rows per language, preview, fit to page, kept per language; HPGL rows B9) (26 functions, S4) | W10-4, W10-10, W10-11 |
| `w_print.c` | 1542 | print panel | `ui-egui/print_dialog`, `cmd/print` | replaced: Print panel with preview, PDF to the system print dialog; all options kept, printcap not needed (B5) | B5 |
| `w_file.c`, `w_dir.c`, `w_browse.c`, `w_listwidget.c` | 3.5 k | file panel and browser with preview | native dialogs (`rfd`) | modern; Open of a missing name starts a new figure; preview and offset fields missing (S4) | W10-12 |
| `w_library.c` | 1831 | library panel | — | missing | D6 |
| `w_srchrepl.c` | 966 | search/replace, spell check | `dialogs::search` | partial: spell check | F5 |
| `w_style.c` | 1026 | style manager | `ui-egui/src/styles.rs`, `engine/src/cmd/styles.rs` | replaced | E6 |
| `w_fontpanel.c` | 424 | font menus | `attributes` Text Font panel | replaced by the Text Font pop-up (S4) | E11 |
| `w_help.c` | 219 | Help menu | menus | partial: reference/how-to entries (S4) | W10-2 |
| `w_capture.c` | 676 | screen capture into a picture | — | missing (through the OS screenshot facilities) | D11 |
| `w_digitize.c` | 471 | digitising tablet | `ui-egui/src/digitize.rs`, `engine/cmd/digitize.rs` | replaced: exports the points of lines as numbered files (S4-8); tablet input not planned | S4-8 |
| `w_drawprim.c`, `w_fontbits.c`, `w_rottext.c`, `w_i18n.c` | 4.6 k | X drawing primitives, bitmap fonts, rotated X text, X input methods | renderer, `figbase/text` (via `text`) | n/a | — |
| `w_setup.c`, `w_util.c` | 2.1 k | X widget plumbing | — | n/a | — |
| `main.c`, `xfigmain.c`, `resources.c`, `mode.c`, `object.c` | 5.1 k | command-line options, resources, mode state | `apps/rustfig`, `Indicator` | partial: options and resources | S3 |

## fig2dev (≈50 k lines)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `fig2dev.c` | 1136 | command line, option parsing per language, driver dispatch | `fig2dev:crates/fig2dev/src/options.rs` (option table), `fig2dev:crates/fig2dev/src/gendev.rs` (`convert`), `crates/engine/src/cmd/file.rs` (`driver_args`: RustFig's settings → option letters, as `u_print.c`), `apps/cli` `convert --option` | ported: options of the languages written today; driver dispatch is `drivers::driver`, reached from `file.export` through `engine/fig2dev` `export`; the whole command line (`get_args`, `main`, `help_msg`, messages, exit codes) in `fig2dev:apps/fig2dev` | B0, B10 |
| `iso2tex.c` | 236 | Latin-1 characters to TeX sequences | `fig2dev/util/iso2tex` | ported | B9 |
| `creationdate.c` | 114 | creation date for reproducible output (`SOURCE_DATE_EPOCH`) | `fig2dev/util/creationdate` | ported | B9 |
| `localmath.c`, `messages.c` | 114 | math helpers, error messages | `geom`, errors; `fig2dev:apps/fig2dev` (messages) | n/a | — |
| `read.c` | 2048 | Fig reader (second reference) | `fig2dev/read` + `crates/format/src/bridge/to_model.rs` (port: the only reader, for every export, the standalone fig2dev and the app, which opens every Fig version through it, `format/fig_read`); texts and comments keep their input bytes beside the Unicode decoding (`Text::raw`, `Comment`); its repairs and checks are functions (`read::repair_line`, `repair_arc`, `*_invalid`, DD7) that `format/bridge/export.rs` calls | ported | B9, G1 |
| `read1_3.c` | 549 | Fig 1.3 reader | `fig2dev/read/read1_3` | ported | B9 |
| `bound.c` | 1156 | bounds incl. arrows and text | `fig2dev:crates/figbase/src/geom/arrow.rs` (arrowheads for the scene); `fig2dev/bound` (port on fig2dev's objects: the bounds of every export; RustFig's own model bounds are gone) | ported (D9, B0); ellipse extents analytic instead of the scan | — |
| `trans_spline.c` | 661 | splines to polylines for exporters | `fig2dev/trans_spline` | ported | B9 |
| `colors.c` | 851 | colour names | `figbase/color/xcolors.rs` (`x_color`: X11 names and `#rgb` forms of `lookup_X_color`), `fig2dev:crates/fig2dev/src/scene/mod.rs` `Rgba::luminance_gray` (`rgb2luminance`) | ported | — |
| `dev/genps.c`, `genpdf.c` | 2.9 k | PostScript/PDF with page layout; the reference where drivers disagree | `drivers/scene.rs` (options, `-F`) + `fig2dev:crates/fig2dev/src/scene/{build,eps,pdf,page,grid}.rs` (`genps_grid`: `-G` for ps, eps, pdf, pstex and the bitmaps); `set_style` → `figbase/style.rs` `LineKind::dash_pattern`, `linewidth_adj` → `figbase::geom::stroke_width`, arrow fills, `PSFONTMAG` → metric text keeps its point size (`scene/build.rs`); object drawing as genps (hidden text, arrows at thickness 0 and pie wedges, single-point lines, closed coincident ends, nonzero pattern fill, no picture frame: `scene/build.rs` `TextOptions::genps`, pass 9); the picture branch (EPS pictures embedded, `readeps.c`) → `fig2dev:crates/fig2dev/src/scene/eps.rs` | partial: UTF-8 text through the CMap and `composefont` (`scene/eps.rs`), PDF Type 3 fonts for text beyond WinAnsi (`scene/pdf.rs`); previews (`-A`/`-T`/`-C`) and `-B`/`-R` done in `scene/preview.rs` (B4b); page layout done (`scene/page.rs`: paper table, `-z -l -p -c -e -x -y -M -o -O`, PDF `-P -Y`) | B4b, B11 |
| `dev/genpstex.c` | 176 | PS/PDF + LaTeX overlay (pstex, pdftex) | `fig2dev/drivers/pstex` | ported | B2 |
| `dev/gentikz.c`, `gentikz.h` | 1870 | TikZ | `fig2dev/drivers/tikz` | ported: byte-identical to fig2dev 3.2.9a on the fixtures, the suite's figures and 285 of 300 sample figures (the rest: `\ooo`-escaped bytes in text); every option, `-G` grid | B1 |
| `dev/genpict2e.c` | 2549 | pict2e | `fig2dev/drivers/pict2e` (text through `util/texfonts` `put_string_bytes`) | ported: byte-identical to fig2dev 3.2.9a on the fixtures, the suite's figures and 285 of the 300-figure sample (the rest: `\ooo` escapes in texts, which the reader decodes) | B3 |
| `dev/genpstricks.c` | 2880 | PSTricks | `fig2dev/drivers/pstricks` | ported: byte-identical to fig2dev 3.2.9a on the fixtures, the suite's figures and 285 of the 300-figure sample (the rest: `\ooo`-escaped bytes in texts); `-p` converts bitmaps to EPS in-process (decoder + EPS writer), no `bmeps` | B3 |
| `dev/genepic.c`, `gentpic.c`, `genpic.c`, `picfonts.h`, `picpsfonts.h` | 3.5 k | epic/eepic/eepicemu, tpic, pic | `fig2dev/drivers/{epic,tpic,pic}` | ported | B9 |
| `dev/genlatex.c`, `genpictex.c`, `gentextyl.c` | 2.9 k | LaTeX picture, PiCTeX, TeXtyl | `fig2dev/drivers/{latex,pictex,textyl}` | ported | B9 |
| `dev/gensvg.c` | 1331 | SVG | `fig2dev:crates/fig2dev/src/scene/svg.rs` (text: Symbol/Dingbats tables, special-text tspans, `-z` ported, audit pass 17); `figbase::geom::stroke_width` ported from `linewidth_adj` | own markup; text ported; reference-tested (A8) | review |
| `dev/genbitmaps.c` | 593 | GIF, JPEG, PNG, PPM, TIFF, XBM, XPM via ghostscript | `drivers/scene.rs` + `fig2dev:crates/fig2dev/src/scene/{bitmaps,raster}.rs` (own rasteriser, no ghostscript) | ported; PNG opaque white, XBM Floyd-Steinberg, PCX 1-4 planes/8-bit/24-bit, `-S`/`-t` messages as C; no `sld` | B6 |
| `dev/psprolog.h` | 936 | PostScript prolog, the 22 fill patterns, the Xfig-UTF8-H CMap | `fig2dev:crates/fig2dev/src/scene/pattern.rs` (`FILL_PAT01`…`22` ported), `scene/ps_cmap.rs` (`XFIG_CMAP1-3`) | patterns and CMap ported | — |
| `dev/genbox.c` | 78 | LaTeX box | `fig2dev:crates/fig2dev/src/scene/latex_box.rs` | ported | — |
| `dev/gencgm.c`, `genibmgl.c`, `gendxf.c` | 3 k | CGM, HPGL (`ibmgl`), DXF | `fig2dev/drivers/{cgm,ibmgl,dxf}` | ported | B9 |
| `dev/genmp.c`, `genmf.c`, `genge.c`, `genmap.c` | — | MetaPost, METAFONT, ge, HTML image maps | `fig2dev:crates/fig2dev/src/drivers/{mp,mf,ge,map}.rs` | ported | B9 |
| `dev/genemf.c` | `fig2dev:crates/fig2dev/src/drivers/emf.rs` | EMF (all objects; texts as UTF-16 of the input bytes, PNG pictures as in C; CJK locale fonts via iconv not ported) | byte-identical to fig2dev on 300 of 300 sample figures | ported | B9 |
| `dev/gentk.c`, `genptk.c` | 2.9 k | Tk, Perl-Tk (`-L tk`, `-L ptk`) | `fig2dev:crates/fig2dev/src/drivers/tk.rs` | ported (byte-identical to fig2dev on 299 of 300 sample figures; stack garbage after a dash of length 0 not reproduced) | B9 |
| `dev/gengbx.c` | 0.9 k | Gerber (`-L gbx`) | `fig2dev:crates/fig2dev/src/drivers/gbx.rs` | ported | B9 |
| `dev/genshape.c` | 0.9 k | LaTeX shape macros (`-L shape`) | `fig2dev:crates/fig2dev/src/drivers/shape.rs` | ported | B9 |
| `dev/encode.c` | 236 | ASCII85 and deflate streams for PostScript/PDF | `fig2dev:crates/fig2dev/src/scene/{eps,pdf}.rs` | own | review |
| `dev/tkpattern.c` | 122 | Tk fill patterns | `fig2dev:crates/fig2dev/src/util/tkpattern.rs` | ported | B9 |
| `dev/xtmpfile.c` | 150 | temporary files | — | n/a | — |
| `dev/read*.c`, `readpics.c` | 3 k | pictures embedded in exports | `fig2dev/util/readpics` (`image` crate; EPS/PDF boxes, PCX, XBM, XPM, .gz/.Z); the canvas (`engine/import.rs`) loads through it first (DD5) | ported | B9, DD5 |
| `dev/psfonts.c`, `texfonts.c`, `setfigfont.c`, `textconvert.c` | 1 k | font names and LaTeX font selection for exports | `fig2dev/util/{psfonts,texfonts,setfigfont,textconvert}` (`-E` charsets beyond Latin-1/-2 via `encoding_rs`, audit pass 27; texts as the input's bytes, `put_string`, conversions; 2026-10-18) | ported | B9 |

## Revision history

| Date | Note |
|------|------|
| 2026-10-14 | First version. |
| 2026-10-15 | W8: xfig's bindings audited (`keys-parity.md`); `CompKeyDB` ported. |
| 2026-10-15 | B9 wave 1: `read.c`, `read1_3.c`, `bound.c`, `trans_spline.c`, `iso2tex.c`, `creationdate.c`, `dev/` font and picture helpers ported into `rustfig-fig2dev`; `genbitmaps.c` (B6). |
| 2026-10-15 | `fig2dev.c` command line ported as `fig2dev:apps/fig2dev` (B10). |
| 2026-10-17 | B3: `genpict2e.c` ported (`drivers/pict2e.rs`); `genpstricks.c` on a row of its own. |
| 2026-10-17 | B3: `genpstricks.c` ported (`drivers/pstricks.rs`). |
| 2026-10-18 | B9: `genemf.c`, `gentk.c`, `genptk.c`, `tkpattern.c` ported. |
| 2026-10-21 | DD9: rows `read.c`, `bound.c`, `f_read.c` checked against DD1–DD7; `engine/fig2dev/to_model` path corrected to `format/bridge`. |
| 2026-10-20 | DD1: `to_model` moved to `crates/format/src/bridge/` (with the new `from_model`). |
| 2026-10-16 | B12: paths after the move (geometry, fonts, colours → `fig2dev:crates/figbase`; EPS/PDF/SVG/bitmap writers, rasteriser, patterns → `fig2dev:crates/fig2dev/src/scene/`; options, languages → `fig2dev:crates/fig2dev/src`; `to_model` → `crates/engine/src/fig2dev/`). |
| 2026-10-16 | `genpstex.c` ported (B2); `readeps.c` and genps's picture branch in `render/eps` (now `fig2dev:crates/fig2dev/src/scene/eps.rs`); `read.c` the only reader for every fig2dev language and for Fig 1.3–2.1 in the app (G1). |
| 2026-10-15 | `fig2dev.c` options and `colors.c` ported (B0). |
| 2026-10-14 | S1: draw and save order ported (`u_redraw.c`, `f_save.c`). |
| 2026-10-14 | Attribute panel: xfig's pictures (`w_icons.c`), xfig's arrow-mode order and grid choices. |
| 2026-10-14 | Mode panel: xfig's icon bitmaps and mode order (`w_icons.c`, `w_modepanel.c`). |
| 2026-10-14 | W7: accelerators from `app-defaults/Fig` and the mode tooltips (`w_modepanel.c`). |
| 2026-10-14 | W4: object snaps ported (`w_snap.c`, `w_intersect.c`). |
| 2026-10-14 | W6: rulers and units box ported (`w_rulers.c`, `u_pan.c`). |
| 2026-10-14 | W5: attribute panel ported (`w_indpanel.c`, masks from `w_modepanel.c`). |
| 2026-10-14 | S2: mouse indicator ported (`w_mousefun.c`, `set_mousefun` wording); notes from the real xfig. |
| 2026-10-21 | S4: every xfig function audited (`source-audit.md`); file statuses of `d_*`, `e_*`, `f_*`, `u_*` and `w_*` rows corrected (f_readpcx/xbm/xpm and f_wrpcx/png, u_ghostscript, u_error are done or replaced, not missing). |
| 2026-10-21 | DD7: `read.c`'s repairs and checks exposed as functions, used by the bridge's export. |
| 2026-10-21 | B15: `f_readeps.c`, `readeps.c` (`read_pdf`, `pdftops`, `gsexe`) and the picture part of `genps.c` now covered by `util/{pdfpic,pdfps,gs}.rs`. |
| 2026-10-22 | D4: `e_tangent.c` ported (`cmd/tangent.rs`, `tools/tangent.rs`), with `smart_next_{arc,ellipse,line,spline}_found` and `close_to_{ellipse,arc,polyline,spline,vector}`. |
| 2026-10-22 | D15: `e_chop.c` extended beyond xfig (`cmd/outline.rs`, `cmd/chop_ext.rs`). |
| 2026-10-22 | D3: `e_chop.c` ported (`cmd/chop.rs`, `tools/chop.rs`); `w_intersect.c`'s intersection math in `geom/intersect.rs`. |
| 2026-10-23 | S4-7: `w_mousefun.c` Shift/Ctrl variants done. |
