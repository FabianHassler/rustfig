# Target-app parity assessment: RustFig vs xfig 3.2.9

**Status:** 2026-10-14, overall **≈61 %** (W4 landed). Revised whenever a task lands.

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
| Editing modes | 12 | 70 % | anchor point for flip/rotate (right button), array copy, align to canvas, figure comments from Edit mode, open compound (edit in place), join/split lines, chop, tangent/normal, angle/length/area measure modes, copy to library | 56 |
| Attributes / indicator | 10 | 75 % | colour editor UI, Angle Geom (latex, manhattan, mountain), smart links, copy counts, arrow types 10–14 exact shapes, dash-dot tick styles | 34 |
| File format | 10 | 85 % | reading Fig 1.x/2.x, exact pre-3.2 spline conversion | 24 |
| Export | 12 | 35 % | PostScript (multi-page, print layout), LaTeX family (TikZ, PSTricks, pict2e, eepic), GIF/JPEG/TIFF/PPM/XBM/XPM, CGM/EMF/HPGL/MetaPost/DXF | 120 |
| Text | 10 | 55 % | character map, LaTeX special-text export, exact PostScript font metrics, compose keys, spell check | 60 |
| View & navigation | 8 | 96 % | zoom box (Ctrl+left), pan to origin with Ctrl+middle | 4 |
| Undo | 4 | 100 % | — | 0 |
| Libraries | 5 | 0 % | library browser and placement | 40 |
| Printing | 5 | 10 % | print dialog and printer output | 30 |
| Settings & styles | 5 | 15 % | settings persistence, recent files, style manager, units dialog | 30 |
| Platforms | 7 | 50 % | Windows/Linux verification, web build and file I/O on the web | 40 |
| **Overall** | 100 | **≈61 %** | | **452** |

## Evidence of what ships

- Drawing tools: `crates/engine/src/tools/mod.rs` (38 tools; tests draw polylines, boxes and text and move objects).
- Commands: `crates/engine/src/cmd/*` (125), swept with hostile parameters in `crates/engine/tests/command_sweep.rs`.
- Fig I/O: `crates/format/src/fig_read.rs` and `fig_write.rs`; round trip and fuzzing in `crates/format/tests/`.
- Rendering: `crates/render/` (scene, tiny-skia raster, SVG, EPS, PDF; all 22 fill patterns; arrowheads; dashes).
- UI: `crates/ui-egui/` (mode panel, attribute panel, depth manager, edit/export/search/settings dialogs, menus with shortcuts).
- Automation: control channel `crates/ui-egui/src/control.rs`, MCP `crates/mcp/`, CLI `apps/cli/`.
- Main window: classic xfig layout (`crates/ui-egui/src/panels.rs` `classic_top`), mouse hints from the engine (`Engine::mouse_hints`), headless UI tests in `crates/ui-egui/tests/ui_smoke.rs`.
- Depth panel: `panels::depth_panel` (counts, drag-toggle, All On/Off/Toggle, Gray/Blank), `Engine::depths_changed`; tests in `crates/engine/tests/depths.rs` (picking, selection, buttons, active-layer export) and `ui_smoke.rs` (rows, buttons, drag, click).
- Object snaps: `crates/engine/src/snap.rs` (ported from `w_snap.c`, `w_intersect.c`), unit tests there and `crates/engine/tests/snap.rs` (endpoint, the manual's move-focus-to-midpoint example, hold/release, intersect, tangent); `snap_menu_arms_a_snap_mode` in `ui_smoke.rs`.
- Rulers and units box: `ui-egui/rulers` (ported from `w_rulers.c`, `u_pan.c`), unit tests for the tick tables, `rulers_pan_and_units_box` in `ui_smoke.rs`, `crates/engine/tests/units.rs`.
- Attribute panel: `ui-egui/attributes` (ported from `w_indpanel.c`), tests `attribute_panel_shows_the_modes_attributes` and `attribute_buttons_step_and_choose` in `ui_smoke.rs`.
- Mouse buttons: xfig's three-button semantics per mode and a modern scheme (`MouseScheme`, `tool.mouse_buttons`, View menu); `mouse_hints_match_behaviour` in `crates/engine/src/tools/mod.rs` presses every button in every tool, state and scheme and checks the indicator against what happened; Alt/Ctrl+click map to middle/right (`canvas::tool_button`), tested in `ui_smoke.rs`.
- Verified on macOS by driving the app over the control channel and reviewing screenshots.

## Revision history

| Date | Overall | Note |
|------|--------:|------|
| 2026-10-13 | 58 % | First assessment. |
| 2026-10-13 | 58 % | W1: View & navigation 75 → 80 % (rounds to the same overall). |
| 2026-10-14 | 61 % | W4: View & navigation 92 → 96 % (Snap menu: object snaps). |
| 2026-10-14 | 61 % | W6: View & navigation 88 → 92 % (rulers, units box, Unit menu). |
| 2026-10-14 | 60 % | W5: Attributes 70 → 75 % (xfig's attribute panel per mode, arc type, text step). |
| 2026-10-14 | 60 % | W3: View & navigation 85 → 88 % (depth panel parity, active-layer export). |
| 2026-10-14 | 60 % | W2: View & navigation 80 → 85 %, Drawing 85 → 88 % (freehand, arc by centre), Editing 65 → 70 % (delete region, cut buffer, copy & flip/rotate, update settings, break and tag, scale about corner or centre). |
