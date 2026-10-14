# Source map: xfig and fig2dev → RustFig

**Status:** 2026-10-14. First version, after the decision to port from xfig's
sources (`LICENSE-xfig`). Nothing has been ported line by line yet: every
"own" row was written from the manual and the format description and must be
checked against the C file named in it.

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
   objects created in RustFig. Fix: a sort key in `Scene::build` (S1).
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
| `d_arc.c` | 359 | arc by three points or by centre | `engine/tools` (Arc) | own | review |
| `d_arcbox.c` | 132 | rounded box | `engine/tools` (ArcBox) | own | review |
| `d_box.c` | 133 | box | `engine/tools` (Box) | own | review |
| `d_ellipse.c` | 345 | circles/ellipses by radius or diameter | `engine/tools` | own | review |
| `d_line.c` | 318 | polylines, polygons, freehand, dimension lines | `engine/tools` | partial: dimension lines missing | D10 |
| `d_picobj.c` | 136 | picture box | `engine/tools` (Picture) | own | review |
| `d_regpoly.c` | 149 | regular polygon | `engine/tools` | own | review |
| `d_spline.c` | 195 | splines | `engine/tools` | own | review |
| `d_subspline.c` | 179 | spline helpers | `geom` | own | review |
| `d_text.c` | 1477 | text entry, cursor, compose keys, paste | `engine/tools` (Text) | partial: compose keys, paste, multi-line editing | F3 |

## xfig: editing modes (`e_*`, 17 k lines)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `e_addpt.c` | 368 | add point | `engine/tools` | own | review |
| `e_align.c` | 962 | align compound / canvas, distribute | `cmd/arrange`, tools | partial: align to canvas | D10 |
| `e_arrow.c` | 293 | add/remove arrowheads | tools, `cmd/object` | own | review |
| `e_break.c` | 92 | break compound (and tag) | tools | own | review |
| `e_chop.c` | 890 | chop objects at intersections | — | missing | D3 |
| `e_compound.c` | 294 | open/close compound | — | missing | D1 |
| `e_convert.c` | 420 | line↔spline, box↔arc-box | `cmd/object` | own | review |
| `e_copy.c` | 185 | copy, array copy, copy to cut buffer | tools | partial: array copy | D10 |
| `e_delete.c` | 307 | delete object/region, cut buffer | tools | own | review |
| `e_deletept.c` | 214 | delete point | tools | own | review |
| `e_edit.c` | 5996 | the Edit panel for every object type, figure comments, spline point shapes | `ui-egui/dialogs` | partial: per-type panels, comments, shape factors | W5 |
| `e_flip.c` | 452 | flip, copy & flip, anchor | tools, `cmd/transform` | partial: anchor | D10 |
| `e_glue.c` | 521 | tag and glue compounds | tools | own | review |
| `e_joinsplit.c` | 714 | join/split lines and splines | — | missing | D2 |
| `e_measure.c` | 514 | measure angle, length, area | `object.measure` | partial: modes and running sums | D5 |
| `e_move.c` | 135 | move (constrained) | tools | own | review |
| `e_movept.c` | 845 | move point (constrained, linked) | tools | partial: linked move | D8 |
| `e_placelib.c` | 337 | place library object | — | missing | D6 |
| `e_rotate.c` | 575 | rotate, copy & rotate, anchor, copies count | tools | partial: anchor, number of copies | D10 |
| `e_scale.c` | 1444 | scale box/edge/centre | tools | partial: edge scaling in one dimension | D10 |
| `e_tangent.c` | 159 | tangent/normal lines | — | missing | D4 |
| `e_update.c` | 827 | update object / settings, update mask | tools, `indicator.take_from` | partial: per-attribute update mask | W5 |

## xfig: files (`f_*`, 9.9 k lines)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `f_read.c` | 2034 | Fig 3.x reader (all quirks) | `format/fig_read` | own | review |
| `f_readold.c` | 537 | Fig 1.x/2.x reader | — | missing | G1 |
| `f_save.c` | 665 | Fig 3.2 writer (type order) | `format/fig_write` | own; type order ported | review |
| `f_load.c` | 384 | load, merge | `cmd/file` | own | review |
| `f_util.c` | 1291 | directories, image colours, `~/.xfigrc` | — | missing: settings | S3/E1 |
| `f_picobj.c` | 970 | picture loading and caching | `engine/import` | own | review |
| `f_readeps.c` | 343 | EPS/PDF pictures (bounding box, ghostscript preview) | — | missing | G2 |
| `f_readgif.c`, `f_readjpg.c`, `f_readpng.c`, `f_readppm.c`, `f_readtif.c` | 1.6 k | bitmap pictures | `image` crate | modern | — |
| `f_readpcx.c`, `f_readxbm.c`, `f_readxpm.c` | 790 | PCX, XBM, XPM pictures | — | missing | G2 |
| `f_wrpcx.c`, `f_wrpng.c` | 317 | bitmap writers (for export) | `render/raster` (PNG) | partial | B6 |
| `f_neuclrtab.c` | 596 | colour quantisation for X visuals | — | n/a | — |

## xfig: utilities (`u_*`, 19 k lines)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `u_draw.c` | 2237 | drawing every object type, arrowheads, dash patterns | `render`, `geom` arrows | own; arrow shapes 0–14 to check | D9 |
| `u_draw_spline.c` | 384 | X-spline evaluation | `geom` xspline (from the paper) | own | review (step size) |
| `u_redraw.c` | 731 | redraw order, gray layers | `render::Scene::build` | ported: draw order, gray layers first | — |
| `u_bound.c` | 653 | bounding boxes | `model` | own | review |
| `u_geom.c` | 1072 | normals, slopes, latex/manhattan endpoints, closeness | `geom` | partial: latex/manhattan modes | D7 |
| `u_quartic.c` | 298 | quartic solver (intersections) | — | missing | W4 |
| `u_elastic.c` | 1004 | rubber-band feedback while drawing | `Engine::overlay` | own | review |
| `u_drag.c` | 861 | dragging objects | tools, overlay | own | review |
| `u_markers.c` | 462 | object markers, tag highlight | `ui-egui/canvas` handles | own | review |
| `u_list.c` | 1358 | object lists, depth bookkeeping | `model::Document` | own | — |
| `u_create.c`, `u_free.c` | 1.4 k | allocation | — | n/a | — |
| `u_translate.c`, `u_scale.c` | 314 | translate/scale objects | `model` transforms | own | — |
| `u_convert.c` | 259 | line↔spline conversion | `cmd/object` | own | review |
| `u_colors.c` | 306 | colour tables | `model` colours | own | review |
| `u_fonts.c` | 1264 | PostScript/LaTeX font tables | `text` | own | review |
| `u_search.c` | 1029 | picking objects, points, segments | `Engine::hit_test`, tools | own | review |
| `u_smartsearch.c` | 507 | smart links (lines follow moved boxes) | — | missing | D8 |
| `u_undo.c` | 962 | undo (one level) | `engine/history` (multi-level) | own, superset | — |
| `u_error.c` | 136 | emergency save on crash | `engine/guard` | partial: emergency save | A6 |
| `u_pan.c` | 89 | pan steps | `engine/view` | own | — |
| `u_print.c` | 923 | print and export through fig2dev | `cmd/file` export | partial | B4 |
| `u_ghostscript.c` | 1218 | EPS/PDF preview through ghostscript | — | missing | G2 |
| `u_spawn.c` | 587 | run external programs | — | missing (fig2dev backend B0, image editor) | B0 |

## xfig: window (`w_*`, 44 k lines; rewritten in egui)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `w_cmdpanel.c` | 1860 | main menus | `ui-egui/menus` | own | review |
| `w_modepanel.c` | 905 | mode panel | `panels::mode_panel` | own | review |
| `w_icons.c` | 3270 | mode and attribute icon bitmaps | `ui-egui/icons` (drawn in code) | own | port bitmaps for xfig's look (W5) |
| `w_indpanel.c` | 4865 | attribute (indicator) panel, choice panels | `panels::indicator` | partial | W5 |
| `w_mousefun.c` | 432 | mouse-function indicator | `panels::mouse_indicator`, `Engine::mouse_hints` | ported: layout and wording | — |
| `w_msgpanel.c` | 881 | message panel, lengths while drawing | `panels::message_panel` | partial: show lengths | D10 |
| `w_layers.c` | 903 | depth panel | `panels::depth_panel` | own | review (Front/Back labels) |
| `w_rulers.c` | 1479 | rulers and units box | — | missing | W6 |
| `w_snap.c`, `w_intersect.c` | 2.8 k | Snap menu (object snaps) | `indicator` positioning only | partial | W4 |
| `w_keyboard.c` | 518 | type coordinates for the next point | — | missing | W4 |
| `w_zoom.c` | 202 | zoom panel | `cmd/view` | partial: zoom panel | E3 |
| `w_grid.c` | 238 | grid drawing | canvas | own | — |
| `w_canvas.c` | 1218 | canvas input, key bindings | `ui-egui/canvas` | own | review |
| `w_cursor.c` | 144 | cursors per mode | canvas | partial | W5 |
| `w_color.c` | 2592 | colour panel and editor | indicator combo | partial: colour editor | E4 |
| `w_export.c` | 2054 | export panel | `dialogs::export` | partial | B7 |
| `w_print.c` | 1542 | print panel | — | modern: PDF to the system print dialog | B5 |
| `w_file.c`, `w_dir.c`, `w_browse.c`, `w_listwidget.c` | 3.5 k | file panel and browser with preview | native dialogs (`rfd`) | modern; preview missing | — |
| `w_library.c` | 1831 | library panel | — | missing | D6 |
| `w_srchrepl.c` | 966 | search/replace, spell check | `dialogs::search` | partial: spell check | F5 |
| `w_style.c` | 1026 | style manager | — | missing | E6 |
| `w_fontpanel.c` | 424 | font menus | indicator | partial | W5 |
| `w_help.c` | 219 | Help menu | menus | own | — |
| `w_capture.c` | 676 | screen capture into a picture | — | n/a (system screenshot) | — |
| `w_digitize.c` | 471 | digitising tablet | — | n/a | — |
| `w_drawprim.c`, `w_fontbits.c`, `w_rottext.c`, `w_i18n.c` | 4.6 k | X drawing primitives, bitmap fonts, rotated X text, X input methods | renderer, `text` | n/a | — |
| `w_setup.c`, `w_util.c` | 2.1 k | X widget plumbing | — | n/a | — |
| `main.c`, `xfigmain.c`, `resources.c`, `mode.c`, `object.c` | 5.1 k | command-line options, resources, mode state | `apps/rustfig`, `Indicator` | partial: options and resources | S3 |

## fig2dev (≈50 k lines)

| File | Lines | What it does | RustFig | Status | Task |
|---|---:|---|---|---|---|
| `fig2dev.c` | 1136 | command line, option parsing per language, driver dispatch | `apps/cli` `convert` | partial: per-language options | B0 |
| `iso2tex.c` | 236 | Latin-1 characters to TeX sequences | — | missing | B1–B3 |
| `creationdate.c` | 114 | creation date for reproducible output (`SOURCE_DATE_EPOCH`) | — | missing | B4 |
| `localmath.c`, `messages.c` | 114 | math helpers, error messages | `geom`, errors | n/a | — |
| `read.c` | 2048 | Fig reader (second reference) | `format/fig_read` | own | review |
| `read1_3.c` | 549 | Fig 1.3 reader | — | missing | G1 |
| `bound.c` | 1156 | bounds incl. arrows and text | `render` | own | review |
| `trans_spline.c` | 661 | splines to polylines for exporters | `geom` | own | review |
| `colors.c` | 851 | colour names | `model` | own | — |
| `dev/genps.c`, `genpdf.c` | 2.9 k | PostScript/PDF with page layout | `render/eps`, `render/pdf` | partial: page layout, multi-page | B4 |
| `dev/genpstex.c` | 176 | PS/PDF + LaTeX overlay (pstex, pdftex) | — | missing | B2 |
| `dev/gentikz.c` | 1870 | TikZ | — | missing | B1 |
| `dev/genpict2e.c`, `genlatex.c`, `genepic.c`, `genpictex.c`, `genpstricks.c` | 9.1 k | LaTeX picture languages | — | missing | B3 (pict2e, PSTricks), B0 (the rest) |
| `dev/gensvg.c` | 1331 | SVG | `render/svg` | own | review |
| `dev/genbitmaps.c` | 593 | GIF, JPEG, PNG, PPM, TIFF, XBM, XPM via ghostscript | `render` PNG | partial | B6 |
| `dev/genemf.c`, `gencgm.c`, `gendxf.c`, `genibmgl.c`, `genmp.c`, `genmf.c`, `genmap.c`, `gentk.c`, `genptk.c`, `genshape.c`, `gentextyl.c`, `gentpic.c`, `genpic.c`, `gengbx.c`, `genge.c`, `genbox.c` | ≈17 k | EMF, CGM, DXF, HPGL, MetaPost, METAFONT, image maps, Tk, shapes, … | — | missing (exotic) | B0 |
| `dev/encode.c` | 236 | ASCII85 and deflate streams for PostScript/PDF | `render/eps`, `render/pdf` | own | review |
| `dev/tkpattern.c` | 122 | Tk fill patterns | — | missing | B0 |
| `dev/xtmpfile.c` | 150 | temporary files | — | n/a | — |
| `dev/read*.c`, `readpics.c` | 3 k | pictures embedded in exports | `image` crate | modern | — |
| `dev/psfonts.c`, `texfonts.c`, `setfigfont.c`, `textconvert.c` | 1 k | font names and LaTeX font selection for exports | `text` | partial | B8 |

## Revision history

| Date | Note |
|------|------|
| 2026-10-14 | First version. |
| 2026-10-14 | S1: draw and save order ported (`u_redraw.c`, `f_save.c`). |
| 2026-10-14 | S2: mouse indicator ported (`w_mousefun.c`, `set_mousefun` wording); notes from the real xfig. |
