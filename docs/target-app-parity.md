# Target-app parity assessment: RustFig vs xfig 3.2.9

**Status:** 2026-10-21 (W16, W17: one mouse scheme and one layout, scores unchanged; save query for open compounds); 2026-10-21 (D1, D2: open compound and join/split, Editing 70 → 75 %); 2026-10-21 (B15: PDF/EPS pictures, Export 66 → 68 %, overall ≈65 %); 2026-10-21 (DD9: evidence paths after the dedup series; scores unchanged); 2026-10-18, overall **≈65 %** (B1, B3, B9: TikZ, PSTricks, EMF, Tk/Perl-Tk; G1 old Fig files; B2 pstex/pdftex; every ported fig2dev language exported from the app; B12 every export through fig2dev; B3 pict2e). Revised whenever a task lands.

## Methodology

The reference is xfig 3.2.9 as described in its public user manual
(`old/xfig-3.2.9a/doc/html`) and the Fig format description (`FORMAT3.2`),
plus fig2dev's export languages. Each area gets a 0–100 % score for the share
of xfig's user-visible behaviour that RustFig reproduces. The overall figure
is the weighted mean. Weights reflect how much everyday xfig use depends on
the area. "Hours" estimates the remaining work to reach 100 %.

Calibration: 100 % means a user of the area notices no missing function;
50 % means the common half works; features that go beyond xfig don't raise
the score.

## Areas

| Area | Weight | Score | Missing | Hours |
|------|-------:|------:|---------|------:|
| Drawing modes | 12 | 88 % | library objects; dimension lines (Shift+left in polyline mode); xfig's click-to-close rules for some shapes | 14 |
| Editing modes | 12 | 75 % | anchor point for flip/rotate (right button), array copy, align to canvas, figure comments from Edit mode, chop, tangent/normal, angle/length/area measure modes, copy to library | 30 |
| Attributes / indicator | 10 | 77 % | colour editor UI, Angle Geom (latex, manhattan, mountain), smart links, copy counts | 30 |
| File format | 10 | 92 % | exact pre-3.2 spline conversion | 4 |
| Export | 12 | 68 % | `sld`, texts with `\ooo` octal escapes (raw bytes, B11 pass 5), the audit's hardening passes (B11), arrows on pie-wedge arcs and file arc centres lost on the way through the Fig writer (gap #25), picture frames and GIF transparency (gap #26) | 90 |
| Text | 10 | 55 % | character map, LaTeX special-text export, exact PostScript font metrics, compose keys, spell check | 60 |
| View & navigation | 8 | 96 % | zoom box (Ctrl+left), pan to origin with Ctrl+middle | 4 |
| Undo | 4 | 100 % | — | 0 |
| Libraries | 5 | 0 % | library browser and placement | 40 |
| Printing | 5 | 10 % | print dialog and printer output | 30 |
| Settings & styles | 5 | 15 % | settings persistence, recent files, style manager, units dialog | 30 |
| Platforms | 7 | 50 % | Windows/Linux verification, web build and file I/O on the web | 40 |
| **Overall** | 100 | **≈65 %** | | **423** |

## Evidence of what ships

- Open compound and join/split (D1, D2): `crates/model/src/lib.rs` (`Document::open`), `crates/engine/src/cmd/{compound,joinsplit}.rs`, `tools/compound.rs`; tests `crates/engine/tests/{open_compound,join_split}.rs`.
- Drawing tools: `crates/engine/src/tools/mod.rs` (40 tools; tests draw polylines, boxes and text and move objects).
- PDF/EPS pictures (B15): `fig2dev:crates/fig2dev/src/util/{pdfpic,pdfps,gs}.rs`, tests `tests/pdf_pictures*.rs` there (against the C fig2dev), canvas in `crates/engine/src/import.rs`, hostile files in `crates/engine/tests/import_fuzz.rs`.
- Commands: `crates/engine/src/cmd/*`, swept with hostile parameters in `crates/engine/tests/command_sweep.rs`.
- Fig I/O: `crates/format/src/fig_read.rs` (fig2dev's reader through `bridge/`), `fig_write.rs`; round trip and fuzzing in `crates/format/tests/`.
- Rendering: the canvas scene in `crates/render/src/lib.rs`; fig2dev's scene, tiny-skia rasteriser, EPS, PDF, SVG and bitmap writers in `fig2dev:crates/fig2dev/src/scene/` (all 22 fill patterns; arrowheads; dashes); geometry, fonts and colours shared through `fig2dev:crates/figbase/`; line widths as fig2dev (`figbase::geom::stroke_width`).
- Fill patterns as fig2dev's PostScript (`fig2dev:crates/fig2dev/src/scene/pattern.rs`, from `psprolog.h`).
- Export bounds as fig2dev's (`fig2dev:crates/fig2dev/src/bound.rs`, ported from `bound.c`): bounding boxes equal fig2dev's on 2,532 of 2,568 sample figures.
- True curves in SVG, PDF and EPS: `fig2dev:crates/figbase/src/geom/curves.rs`, `SubPath::curve`; test `ellipses_export_as_curves`.
- fig2dev's options: `fig2dev:crates/fig2dev/src/options.rs`, parsed by fig2dev's drivers; RustFig's settings become option letters in `crates/engine/src/cmd/file.rs` (`driver_args`, as xfig's `u_print.c`); tests `fig2dev_options_change_the_output`, `depth_options_select_objects_and_bounds`, `bad_options_are_refused` in `crates/engine/tests/export_languages.rs`; bounding boxes with `-b`, `-m`, `-Z`, `-D -K` checked equal to fig2dev's.
- TikZ, PSTricks, pict2e, EMF, Tk/Perl-Tk (B1, B3, B9): `fig2dev:crates/fig2dev/src/drivers/{tikz,pstricks,pict2e,emf,tk}.rs`, `util/tkpattern.rs`; byte-identical to fig2dev 3.2.9a on the fixtures, the suite's figures and ~285 of 300 sample figures (rest: octal-escape texts); fig2dev's suite 79 pass / 9 fail / 1 skipped / 0 unported.
- Bitmap languages (B6): `fig2dev:crates/fig2dev/src/scene/bitmaps.rs`; tests in `crates/engine/tests/export_bitmaps.rs`.
- fig2dev port (B9): `fig2dev:crates/fig2dev` (reader, bounds, spline conversion, helpers); tests in `fig2dev:crates/fig2dev/tests/`.
- One reader for every fig2dev language (`fig2dev:crates/fig2dev/src/read.rs`; `crates/format/src/bridge/to_model.rs` for the app); every Fig file (1.3–3.2) opens in the app through it (`crates/format/src/fig_read.rs`, 83 lines, tests `crates/engine/tests/fig2dev_bridge.rs`); every ported language exports from the engine, the export dialog and `rustfig-cli`.
- fig2dev as a program of its own (B12): every export hands the bridge's objects (`crates/format/src/bridge/export.rs`, DD7; Fig text only as fallback) to `fig2dev::convert` (`crates/engine/src/fig2dev/mod.rs` `export`); `fig2dev` and `fig2dev:apps/fig2dev` depend only on `figbase` (`cargo xtask layers`).
- pict2e (B3): `fig2dev:crates/fig2dev/src/drivers/pict2e.rs`, byte-identical to fig2dev 3.2.9a on the fixtures, fig2dev's suite figures and 285 of the 300-figure corpus sample with 13 option sets; golden tests in `fig2dev:crates/fig2dev/tests/pict2e.rs`.
- pstex/pdftex (B2): `fig2dev:crates/fig2dev/src/drivers/pstex.rs`, the `_t` file byte-identical to fig2dev's on a test figure; pictures in exports through `fig2dev:crates/fig2dev/src/util/readpics.rs`, EPS pictures embedded by `fig2dev:crates/fig2dev/src/scene/eps.rs`.
- Standalone `fig2dev` (B10): `fig2dev:apps/fig2dev` with fig2dev 3.2.9a's command line; tests in `fig2dev:apps/fig2dev/tests/cli.rs`; fig2dev's own suite in `fig2dev:apps/fig2dev/tests/fig2dev_suite.rs` (opt-in): 73 pass, 9 fail, 1 skipped, 6 need unported languages (2026-10-17, with pict2e).
- Export languages: `fig2dev:crates/fig2dev/src/languages.rs` (xfig's list and suffixes), `drivers/latex_box.rs`, `file.export_formats`; tests in `crates/engine/tests/export_languages.rs` and `export_dialog_lists_xfigs_languages` in `ui_smoke.rs`.
- Corpus comparison with fig2dev's EPS and PDF on xfig's ≈2,550 sample figures: `crates/engine/tests/fig2dev_corpus.rs` (opt-in), baseline `fixtures/corpus-scores.json`, report `target/fig2dev-reference/corpus/`.
- Reference comparison with fig2dev: `crates/engine/tests/fig2dev_reference.rs`, 11 fixtures with recorded scores (`fixtures/scores.json`); arrowheads, arcs and dashes ported from fig2dev (D9): `geom/arrow.rs`, `model` `dash_pattern`.
- UI: `crates/ui-egui/` (mode panel, attribute panel, depth manager, edit/export/search/settings dialogs, menus with shortcuts).
- Automation: control channel `crates/ui-egui/src/control.rs`, MCP `crates/mcp/`, CLI `apps/cli/`.
- Main window: classic xfig layout (`crates/ui-egui/src/panels.rs` `classic_top`), mouse hints from the engine (`Engine::mouse_hints`), headless UI tests in `crates/ui-egui/tests/ui_smoke.rs`.
- Depth panel: `panels::depth_panel` (counts, drag-toggle, All On/Off/Toggle, Gray/Blank), `Engine::depths_changed`; tests in `crates/engine/tests/depths.rs` (picking, selection, buttons, active-layer export) and `ui_smoke.rs` (rows, buttons, drag, click).
- Object snaps: `crates/engine/src/snap.rs` (ported from `w_snap.c`, `w_intersect.c`), unit tests there and `crates/engine/tests/snap.rs` (endpoint, the manual's move-focus-to-midpoint example, hold/release, intersect, tangent); `snap_menu_arms_a_snap_mode` in `ui_smoke.rs`.
- Rulers and units box: `ui-egui/rulers` (ported from `w_rulers.c`, `u_pan.c`), unit tests for the tick tables, `rulers_pan_and_units_box` in `ui_smoke.rs`, `crates/engine/tests/units.rs`.
- Attribute panel (W19: compact, short captions, content-sized buttons; tests `every_mode_fits_in_about_sixty_percent_of_xfigs_width`, `captions_fit_their_buttons`): `ui-egui/attributes` (ported from `w_indpanel.c`), tests `attribute_panel_shows_the_modes_attributes` and `attribute_buttons_step_and_choose` in `ui_smoke.rs`.
- Mouse buttons: xfig's three-button semantics per mode, the only scheme (W16 removed the modern one); `mouse_hints_match_behaviour` in `crates/engine/src/tools/mod.rs` presses every button in every tool and state and checks the indicator against what happened; Alt/Ctrl+click map to middle/right (`canvas::tool_button`), tested in `ui_smoke.rs`.
- Verified on macOS by driving the app over the control channel and reviewing screenshots.

## Revision history

| Date | Overall | Note |
|------|--------:|------|
| 2026-10-21 | ≈65 % | W16, W17: one mouse scheme and one layout; save query for open compounds; no score moved. |
| 2026-10-21 | ≈65 % | DD9: evidence paths after the dedup series (reader, bounds, exports through fig2dev); no score moved. |
| 2026-10-18 | ≈65 % | B1, B3, B9: TikZ, PSTricks, EMF, Tk/Perl-Tk ported (Export 57 → 66 %, 50 h less). |
| 2026-10-17 | ≈64 % | B3: pict2e ported, byte-identical to fig2dev on 285 of 300 sample figures (Export 55 → 57 %, 14 h less). |
| 2026-10-16 | ≈64 % | B12: fig2dev self-contained, every export through `convert` (Export stays 55 %: architecture, no new language). |
| 2026-10-16 | ≈64 % | G1, B2, B9 audit passes: old Fig files read (File format 85 → 92 %); pstex/pdftex and every ported fig2dev language exported from the app (Export 40 → 55 %; hours up with the audit's B11). |
| 2026-10-21 | — | W19: Attributes panel made compact (look only; score unchanged). |
| 2026-10-15 | ≈61 % | W8, W9: xfig's key and mouse bindings; attribute pictures as vectors, arrow types 10–14 selectable (Attributes 75 → 77 %). |
| 2026-10-15 | ≈61 % | B10: standalone fig2dev (Export stays 40 %: no new language). |
| 2026-10-15 | ≈61 % | B0: fig2dev's export options (Export 39 → 40 %). |
| 2026-10-15 | ≈61 % | D12: fill patterns from fig2dev. |
| 2026-10-14 | ≈61 % | B0: fig2dev's export bounds (Export 37 → 39 %). |
| 2026-10-14 | ≈61 % | B0 started: export language table, LaTeX box, pspdf (Export 35 → 37 %). |
| 2026-10-14 | ≈61 % | D9: arrowheads (all 15 types), pie-arc arrows, arc direction, dashes from fig2dev. |
| 2026-10-14 | ≈61 % | A8: fig2dev reference tests; line widths fixed. Exports re-estimated for porting fig2dev (+20 h). |
| 2026-10-13 | 58 % | First assessment. |
| 2026-10-13 | 58 % | W1: View & navigation 75 → 80 % (rounds to the same overall). |
| 2026-10-14 | 61 % | W4: View & navigation 92 → 96 % (Snap menu: object snaps). |
| 2026-10-14 | 61 % | W6: View & navigation 88 → 92 % (rulers, units box, Unit menu). |
| 2026-10-14 | 60 % | W5: Attributes 70 → 75 % (xfig's attribute panel per mode, arc type, text step). |
| 2026-10-14 | 60 % | W3: View & navigation 85 → 88 % (depth panel parity, active-layer export). |
| 2026-10-14 | 60 % | W2: View & navigation 80 → 85 %, Drawing 85 → 88 % (freehand, arc by centre), Editing 65 → 70 % (delete region, cut buffer, copy & flip/rotate, update settings, break and tag, scale about corner or centre). |
| 2026-10-21 | ≈65 % | B15: PDF and EPS picture objects in exports and on the canvas (fig2dev v0.6.0, hayro, optional ghostscript); Export 66 → 68 %. |
