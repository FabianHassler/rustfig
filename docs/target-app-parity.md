# Target-app parity assessment: RustFig vs xfig 3.2.9

2026-10-23 (D6 done: library mode, 30 bundled libraries; Libraries 0 → 85 %); P26-10-23 (W10-13 background export, scores unchanged; D10 done: fractions of an inch in dimension-line lengths removed from Drawing modes' missing items; W10-14 saved versions; scores unchanged); 2026-10-22 (recount after E4, E4b, E6, E10, D16–D18: Text 55 → 65 %, Settings & styles 15 → 75 %, overall ≈65 → ≈73 %, 413 → 379 h); 2026-10-22 (E10: text editing on the canvas; Text score unchanged, evidence added); 2026-10-22 (W10-4, W10-8, W10-9, W10-10, W10-11: Export panel rows per language with preview, kept per language; new file on Open; `.bak` on save; Export 68 %, scores otherwise unchanged); 2026-10-22 (W10-15, W10-17 to W10-19, W10-21: edit panel live, compound rows, point table, pictorial menus; Attributes evidence); 2026-10-22 (W10-16, W10-20: picture edit panel; scores unchanged); 2026-10-22 (D5: measure modes, Editing 80 → 81 %, 18 → 12 h); 2026-10-22 (W10-28, W10-2: bundled manual); 2026-10-22 (D4: tangent/normal, Editing 79 → 80 %, 26 → 18 h); 2026-10-22 (D15: Chop extended, Editing 78 → 79 %; D15 was a new task, so the hours stay); 2026-10-22 (D14: smart links extended, Attributes 83 → 84 %; D14 was a new task, so the hours stay); 2026-10-22 (D7: Angle Geom, Attributes 80 → 83 %; D3: chop, Editing 75 → 78 %; D8: linked move, Attributes 77 → 80 %, 10 h less); 2026-10-21 (W16, W17: one mouse scheme and one layout, scores unchanged; save query for open compounds); 2026-10-21 (D1, D2: open compound and join/split, Editing 70 → 75 %); 2026-10-21 (B15: PDF/EPS pictures, Export 66 → 68 %, overall ≈65 %); 2026-10-21 (DD9: evidence paths after the dedup series; scores unchanged); 2026-10-18, overall **≈65 %** (B1, B3, B9: TikZ, PSTricks, EMF, Tk/Perl-Tk; G1 old Fig files; B2 pstex/pdftex; every ported fig2dev language exported from the app; B12 every export through fig2dev; B3 pict2e). Revised whenever a task lands.

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
| Drawing modes | 12 | 95 % | xfig's click-to-close rules for some shapes | 8 |
| Editing modes | 12 | 83 % | figure comments from Edit mode | 18 |
| Attributes / indicator | 10 | 90 % | mix memory and lock of xfig's colour panel (not built, recent colours instead) | 3 |
| File format | 10 | 92 % | exact pre-3.2 spline conversion | 4 |
| Export | 12 | 68 % | `sld`, texts with `\ooo` octal escapes (raw bytes, B11 pass 5), the audit's hardening passes (B11), arrows on pie-wedge arcs and file arc centres lost on the way through the Fig writer (gap #25), picture frames and GIF transparency (gap #26) | 90 |
| Text | 10 | 70 % | LaTeX special-text export, exact PostScript font metrics, spell check (text editing on the canvas done 2026-10-22, E10; character map 2026-10-22, E5) | 40 |
| View & navigation | 8 | 100 % | — (zoom box, pan to origin and unzoom with Ctrl+click done 2026-10-22, E3) | 0 |
| Undo | 4 | 100 % | — | 0 |
| Libraries | 5 | 85 % | xfig's library list files (a file naming several library folders), the Stop button while a big library loads, the web build (no file system: no bundled libraries yet) | 6 |
| Printing | 5 | 85 % | verification of the printer hand-off on Windows and Linux, browser printing (web), several figures as one job | 6 |
| Settings & styles | 5 | 80 % | — (Global settings rows with effect, Unit/Scale panel, Arrow Size panel and number pop-ups done in W10-23, -24, -31, -32; preferences, recent files, styles with families E6, colours E4 done) | 0 |
| Platforms | 7 | 70 % | Windows/Linux manual checks (checklist in hardware-parity.md), browser check of the web build, autosave in the browser (C3), pictures and Merge on the web | 16 |
| **Overall** | 100 | **≈84 %** | | **337** |

## Evidence of what ships

- Printing (B5, 2026-10-23; Printing 10 → 85 %): `crates/engine/src/print.rs` (settings, fig2dev PDF in page mode as `u_print.c` builds its options, `Spooler` trait with `RecordingSpooler`/`NoSpooler` (the app's `OsSpooler` calls `rustfig_os::print::print_pdf`, B5b: the OS print dialog on macOS, Windows, Linux portal, browser tab; no printer or job-parameter fields; `crates/os/src/print.rs`), page rendering with hayro), commands `print.*` in `cmd/print.rs`, panel `crates/ui-egui/src/print_dialog.rs`, manual `docs/manual/print.md`; tests `crates/engine/tests/print.rs` (paper, orientation, pages, grid, layers, spooler, batch, hostile params), `crates/ui-egui/tests/print_panel_ui.rs`. The macOS dialog is checked by hand; the Windows and Linux dialogs compile but are not tested on hardware; the batch is sent as one job per figure.

- Platform checks (P2, 2026-10-23; Platforms 65 → 70 %): `crates/engine/src/dirs.rs` (settings and recovery folders per OS as functions, tested for all three; Windows falls back to `%USERPROFILE%`, a relative XDG value is ignored); `crates/engine/tests/platform_paths.rs` (command line with any file name, open/save/recent/recovery with non-ASCII paths); `crates/ui-egui/tests/platform_checks.rs` (HiDPI scales 1 to 4, non-ASCII names through the menus, `raster_scale`). Fixed: a canvas wider than the GPU's biggest texture (2048 pixels in the test harness; weak GPUs and 4K/5K screens) drew nothing and raised an internal error every frame; the raster now lowers its resolution (`canvas::raster_scale`). Fixed: `rustfig` and `rustfig-cli` aborted at start on a command-line argument that is not UTF-8 (`std::env::args`), now read lossily. Live on macOS (Retina 2x, control channel): `.fig` with a path of accents, CJK and emoji opened from the command line and saved; screenshot 2560x1652. Mouse buttons are covered by `input.rs` tests (Ctrl+click right, Alt+click middle, left+right chord) and `ui_smoke.rs`.

- Browser file I/O (C2, 2026-10-23; Platforms 55 → 65 %): `crates/ui-egui/src/web_io.rs`; `FileDialogs` gained `is_web`, `pick_web` (async `rfd::AsyncFileDialog`, answer through a `WebInbox`) and `download` (rfd's write starts the browser download); Open reads the picked bytes with `file.open_data {base64}` (also PDF/SVG/PNG exports with the figure embedded, which open as `NAME.fig`), dropped files take the same way (unsaved changes asked first), Save and Save As download `file.save_data`, the Export panel downloads `file.export_data` (several files for PostScript+PDF). Tests: `crates/ui-egui/tests/web_io.rs` (open, edit, save, reopen the download; drop; export; embedded PDF; hostile bytes) on `ScriptedDialogs::web`, `crates/engine/tests/open_data.rs`; `cargo xtask wasm` compiles it.

- Web packaging (C1, 2026-10-23; Platforms 50 → 55 %): `cargo xtask web` runs trunk (`web/index.html`, built from `apps/rustfig`; finds `trunk` on PATH or in cargo's bin folder, prints the install line otherwise; `--debug`, `--serve`), checks that `dist/` holds index.html and the wasm; the page shows "Loading RustFig…" until the app starts. Built here: `dist/` = index.html, 14 MB wasm (release, wasm-opt), JS glue. Not opened in a browser (manual check in `docs/hardware-parity.md`).

- Mode options and saved versions (D10, W10-14, 2026-10-23): dimension-line lengths as xfig's fractions of an inch (`view::length_string`, `UnitScale.decimal_inches`), the Dimension line settings window with example preview, arrow pictures and "none" per end (`crates/ui-egui/src/dimline_panel.rs`); `file.save` keeps versions in the recovery folder (`crates/engine/src/recovery.rs`, `cmd/file.rs`: `file.versions`, `file.restore_version`; test `crates/engine/tests/export_panel.rs`), File > Saved versions… (`ui-egui/src/dialogs.rs`). Leftovers: preview arrowheads are plain triangles, lengths while drawing stay decimal.

- Character map and shortcut sheet (E5, E8; Text 65 → 70 %): `crates/text/src/symbol_tables.rs` (Symbol/ZapfDingbats tables from gensvg.c, code to Unicode and back), `Engine::text_insert_chars` and command `text.insert_char` (`crates/engine/src/{tools/mod.rs,cmd/tool.rs}`), windows `crates/ui-egui/src/{charmap,keys_sheet}.rs` (`app.character_map`, `app.keys`); tests `crates/engine/tests/char_map.rs` (∑ in Symbol exports as `&#8721;` in SVG and `\345` in PostScript), `crates/ui-egui/tests/ui_smoke.rs`.

- Zoom box and shape factor (E3, S4-6; View & navigation 96 → 100 %): `crates/engine/src/tools/ctrl.rs` (`ToolEvent::Ctrl`, `ToolState::ZoomBox`), commands `view.zoom_box` (`cmd/view.rs`) and `spline.shape_factor` (`cmd/sfactor.rs`), `Engine::mouse_hints_for`, canvas `ctrl_modifier` (`crates/ui-egui/src/canvas.rs`); tests `crates/engine/tests/ctrl_click.rs`, `ui_smoke.rs` `xfig_ctrl_key_is_cmd_on_macos`.
- Open compound and join/split (D1, D2): `crates/model/src/lib.rs` (`Document::open`), `crates/engine/src/cmd/{compound,joinsplit}.rs`, `tools/compound.rs`; tests `crates/engine/tests/{open_compound,join_split}.rs`.
- Chop (D3): `crates/engine/src/cmd/chop.rs` (command `object.chop`), `crates/engine/src/tools/chop.rs` (mode `tool.chop`), intersection math in `crates/geom/src/intersect.rs`; tests `crates/engine/tests/chop.rs`.
- S4-1, S4-2, S4-4, S4-5 (scores unchanged): `Engine::locate_press` and `ToolKind::locates` (Shift+click Locate Object, `crates/engine/src/tools/mod.rs`), `ToolState::Paste` (paste follows the pointer), `crates/engine/src/tools/cursor.rs` (cursor per mode), `object.convert` box/arc-box and spline-kind memory; tests `crates/engine/tests/s4_locate_paste_convert.rs`.
- Measure modes (D5): `crates/engine/src/cmd/measure.rs` (commands `object.measure_angle`, `_length`, `_area`, `_reset`; `compute_*` of `u_geom.c`, messages of `w_msgpanel.c`), `crates/engine/src/tools/measure.rs` (modes `tool.angle_measure`, `tool.length_measure`, `tool.area_measure`, keys Ctrl+G/N/M), `crates/engine/tests/measure.rs`, manual `measure.md`.
- Tangent/normal (D4): `crates/engine/src/cmd/tangent.rs` (command `object.tangent_line`, curve geometry from `u_smartsearch.c`/`u_geom.c`), `crates/engine/src/tools/tangent.rs` (mode `tool.tangent`, key `n`), Tangent/Norm Length in `Indicator::tangent_length` and `ui-egui/attributes.rs`; tests `crates/engine/tests/tangent.rs`.
- Trim, split and extend mode (D16): `ToolKind::Trim` (`tool.trim`, key `w`), `trim_event` in `crates/engine/src/tools/chop.rs`, vector icon `"trim"` in `crates/ui-egui/src/mode_icons.rs`; tests `crates/engine/tests/trim_mode.rs`, `ui_smoke.rs`; manual `docs/manual/trim.md`.
- Chop extended (D15): `crates/engine/src/cmd/outline.rs`, `cmd/chop_ext.rs` (commands `object.trim`, `object.split_at`, `object.cut_all`, `object.extend`), the buttons moved to the Trim mode (D16); tests `crates/engine/tests/chop_extended.rs`.
- Drawing tools: `crates/engine/src/tools/mod.rs` (41 tools; tests draw polylines, boxes and text and move objects).
- Global settings, Unit/Scale, Arrow Size, number pop-ups, align and grid choices (W10-23, -24, -31, -32, -33, W10-3): `crates/engine/src/globals.rs` (rows read from the preferences), `cmd/arrange.rs` (`spread`, the seven align choices), `indicator.rs` (`snap_iso`, `arrow_value`), `view.rs` (`apply_integer_zoom`), `crates/ui-egui/src/{dialogs,rulers,attributes,canvas}.rs`, tests `crates/engine/tests/w10_group4_5.rs` and five in `crates/ui-egui/tests/ui_smoke.rs`.
- Picture edit panel (W10-16, W10-20): `crates/engine/src/cmd/picture.rs` (`picture.*` commands), `crates/ui-egui/src/dialogs.rs` (`picture_rows`), tests `crates/engine/tests/picture_panel.rs` and `picture_panel_rows_relink_and_drop` in `crates/ui-egui/tests/ui_smoke.rs`.
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
- Export panel (W10-4, W10-10, W10-11): `crates/engine/src/export_panel.rs` (`rows` = xfig's `manage_optional`, `params` = `print_export`, `fit_page`, `figure_size`), commands in `crates/engine/src/cmd/export_panel.rs`, dialog `crates/ui-egui/src/dialogs.rs` (`export_dialog`); tests `crates/engine/tests/export_panel.rs`, `crates/ui-egui/tests/export_panel_ui.rs`. New file on Open and `.bak` on save (W10-8, W10-9): `cmd/file.rs` (`open`, `backup_file`), test `bak_on_save_and_new_file_on_open`.
- Editable exports (E16, RustFig addition, no score change): `crates/format/src/embed.rs` (PDF incremental update with embedded file, SVG metadata, PNG iTXt), `embed` in `export_panel.rs` and `cmd/file.rs`; tests `crates/engine/tests/embed_export.rs`, `embedded_figure_damaged` in `import_fuzz.rs`, `export_dialog_embed_checkbox_makes_an_editable_pdf` in `ui_smoke.rs`.
- fig2dev port (B9): `fig2dev:crates/fig2dev` (reader, bounds, spline conversion, helpers); tests in `fig2dev:crates/fig2dev/tests/`.
- One reader for every fig2dev language (`fig2dev:crates/fig2dev/src/read.rs`; `crates/format/src/bridge/to_model.rs` for the app); every Fig file (1.3–3.2) opens in the app through it (`crates/format/src/fig_read.rs`, 83 lines, tests `crates/engine/tests/fig2dev_bridge.rs`); every ported language exports from the engine, the export dialog and `rustfig-cli`.
- fig2dev as a program of its own (B12): every export hands the bridge's objects (`crates/format/src/bridge/export.rs`, DD7; Fig text only as fallback) to `fig2dev::convert` (`crates/engine/src/fig2dev/mod.rs` `export`); `fig2dev` and `fig2dev:apps/fig2dev` depend only on `figbase` (`cargo xtask layers`).
- pict2e (B3): `fig2dev:crates/fig2dev/src/drivers/pict2e.rs`, byte-identical to fig2dev 3.2.9a on the fixtures, fig2dev's suite figures and 285 of the 300-figure corpus sample with 13 option sets; golden tests in `fig2dev:crates/fig2dev/tests/pict2e.rs`.
- pstex/pdftex (B2): `fig2dev:crates/fig2dev/src/drivers/pstex.rs`, the `_t` file byte-identical to fig2dev's on a test figure; pictures in exports through `fig2dev:crates/fig2dev/src/util/readpics.rs`, EPS pictures embedded by `fig2dev:crates/fig2dev/src/scene/eps.rs`.
- xfig command line (S3): `crates/engine/src/cmdline.rs` (option table, abbreviations, X resources; tests in the module), `crates/ui-egui/src/settings.rs` (`start_session`: per-run values never written back), `apps/rustfig/src/main.rs` (`-update`, `-geometry`); table in [ui-parity.md](ui-parity.md).
- Standalone `fig2dev` (B10): `fig2dev:apps/fig2dev` with fig2dev 3.2.9a's command line; tests in `fig2dev:apps/fig2dev/tests/cli.rs`; fig2dev's own suite in `fig2dev:apps/fig2dev/tests/fig2dev_suite.rs` (opt-in): 73 pass, 9 fail, 1 skipped, 6 need unported languages (2026-10-17, with pict2e).
- Export languages: `fig2dev:crates/fig2dev/src/languages.rs` (xfig's list and suffixes), `drivers/latex_box.rs`, `file.export_formats`; tests in `crates/engine/tests/export_languages.rs` and `export_dialog_lists_xfigs_languages` in `ui_smoke.rs`.
- Corpus comparison with fig2dev's EPS and PDF on xfig's ≈2,550 sample figures: `crates/engine/tests/fig2dev_corpus.rs` (opt-in), baseline `fixtures/corpus-scores.json`, report `target/fig2dev-reference/corpus/`.
- Reference comparison with fig2dev: `crates/engine/tests/fig2dev_reference.rs`, 11 fixtures with recorded scores (`fixtures/scores.json`); arrowheads, arcs and dashes ported from fig2dev (D9): `geom/arrow.rs`, `model` `dash_pattern`.
- Colour picker and editor (E4, W10-36): `crates/engine/src/cmd/color.rs` (`color.parse`, `color.from_hsv`, `color.names`, `color.new`, `color.edit`, `color.delete`, `color.recent`, `color.use`, `color.eyedropper`), `crates/engine/src/eyedropper.rs`, `crates/ui-egui/src/color_dialog.rs` (Edit > Colours…, the pop-up's recent colours, More… and Eyedropper), `crates/engine/tests/colors.rs`, `colour_picker_hex_input_sets_the_pen_colour` and `colour_editor_creates_edits_and_deletes_user_colours` in `ui_smoke.rs`. E4b: `crates/ui-egui/src/screen_pick.rs` (Pick from screen: macOS NSColorSampler, Windows GetPixel, Linux portal PickColor; unit tests for conversion and result handling).
- Update Control (W10-1): `UpdateFlag`, `Indicator::update_object` / `take_from`, commands `indicator.set_update_flag` and `indicator.update_flags`, `crates/engine/tests/update_control.rs`, `update_control_flags_in_update_mode` in `ui_smoke.rs`.
- UI: `crates/ui-egui/` (mode panel, attribute panel, depth manager, edit/export/search/settings dialogs, menus with shortcuts).
- Automation: control channel `crates/ui-egui/src/control.rs`, MCP `crates/mcp/`, CLI `apps/cli/`.
- Main window: classic xfig layout (`crates/ui-egui/src/panels.rs` `classic_top`), mouse hints from the engine (`Engine::mouse_hints`), headless UI tests in `crates/ui-egui/tests/ui_smoke.rs`.
- Depth panel: `panels::depth_panel` (counts, drag-toggle, All On/Off/Toggle, Gray/Blank), `Engine::depths_changed`; tests in `crates/engine/tests/depths.rs` (picking, selection, buttons, active-layer export) and `ui_smoke.rs` (rows, buttons, drag, click).
- Object snaps: `crates/engine/src/snap.rs` (ported from `w_snap.c`, `w_intersect.c`), unit tests there and `crates/engine/tests/snap.rs` (endpoint, the manual's move-focus-to-midpoint example, hold/release, intersect, tangent); `snap_menu_arms_a_snap_mode` in `ui_smoke.rs`.
- Rulers and units box: `ui-egui/rulers` (ported from `w_rulers.c`, `u_pan.c`), unit tests for the tick tables, `rulers_pan_and_units_box` in `ui_smoke.rs`, `crates/engine/tests/units.rs`.
- System clipboard (E7): the cut buffer is also published through `PictureClipboard::set_objects` (`engine/src/picture.rs`, `cmd/edit.rs` `publish_cut_buffer`, `sync_from_system_clipboard`, `picture_from_image`), the native clipboard in `ui-egui/src/clipboard.rs`; tests in `crates/engine/tests/clipboard_objects.rs` (fake clipboard, two engines).
- Edit panel (W10-15, W10-17 to W10-19, W10-21): `ui-egui/dialogs.rs` `edit_dialog`, `live_apply`, `point_table`, `parse_points` (ports `e_edit.c` `make_window_compound`, `collapse_depths`, `get_new_compound_values`), pictorial menus `ui-egui/attributes.rs` (`line_menu` and others), engine `object.set_geometry` (compound corners, `resize`) and `object.set_attributes` (`texts`); tests `compound_corners_reposition_and_rescale_like_xfig`, `point_table_resizes_lines_and_splines` in `engine/tests/edit_panel.rs`, `live_edit_panel_is_one_undo_step_and_cancel_restores`, `compound_panel_repositions_rescales_collapses_and_edits_texts`, `point_table_edits_points`, `edit_panel_has_pictorial_menus` in `ui_smoke.rs`.
- Attribute panel (W19: xfig's layout, buttons narrowed to their content): `ui-egui/attributes` (ported from `w_indpanel.c`), tests `attribute_panel_shows_the_modes_attributes` and `attribute_buttons_step_and_choose` in `ui_smoke.rs`.
- Angle Geom (D7): `crates/engine/src/anglegeom.rs` (ports `u_geom.c`'s `latex_endpoint`, `get_slope` and `u_elastic.c`'s `constrainedangle_line`), `Engine::angle_constrained` / `angle_anchor` in `tools/mod.rs`, the Angle Geom button in `ui-egui/attributes`; tests `crates/engine/tests/angle_geom.rs`.
- Copy counts (D10): Num Copies (rotate modes, `I_ROTATE`), Num X / Num Y Copies (Copy mode, `I_COPY`) in `Indicator` and `ui-egui/attributes.rs`; engine `edit.copy_array`, `transform.rotate` `copies`; tests in `crates/engine/tests/mode_options.rs`, `ui_smoke.rs`.
- Text editing (E10): `crates/engine/src/tools/textedit.rs` (word ranges, selection buffer, caret under the pointer), `tools/mod.rs` `text_event` (`ToolEvent::Nav`, `Preedit`, `TripleClick`), `text_selection`/`text_cut` for the clipboard, caret, selection and IME underline in `ui-egui/canvas.rs` `draw_caret`, keys in `menus.rs` `text_key`; tests `text_*` in `tools/mod.rs`.
- Linked move (D8): `crates/engine/src/links.rs` (ports `u_list.c`'s `get_links`, `point_on_perim`, `get_interior_links`, `point_on_inside`, `adjust_links`); the Smart Links indicator button in `ui-egui/attributes`; tests `crates/engine/tests/linked_move.rs`. D14 extends it: `links.rs` targets circles, ellipses, arcs, polygons, closed splines, text boxes (distance to the outline) beside boxes and compounds, follows with open polylines and splines, links both ends of a line, and every operation (`transform.apply`, `links::translate_each`, `edit.duplicate` copy with links, `point.move` box resize, `transform.nudge`); previews and end highlights in `tools/mod.rs`; tests `crates/engine/tests/linked_ext.rs`.
- Mouse buttons: xfig's three-button semantics per mode, the only scheme (W16 removed the modern one); `mouse_hints_match_behaviour` in `crates/engine/src/tools/mod.rs` presses every button in every tool and state and checks the indicator against what happened; Alt/Ctrl+click map to middle/right (`canvas::tool_button`), tested in `ui_smoke.rs`.
- Digitize, relative picture names, Autorefresh, New window (S4-3, S4-8, S4-9): `crates/engine/src/cmd/digitize.rs` (`file.digitize`), `file.save` picture rebasing in `cmd/file.rs`, `crates/ui-egui/src/digitize.rs` and `reload.rs`; tests `relative_pictures.rs`, `digitize.rs`, `digitize_with_hostile_params` (`command_sweep.rs`), `digitize_window_writes_selected_line` and `autorefresh_toggle_and_new_window_entries` (`ui_smoke.rs`), unit tests in `reload.rs`.
- Libraries (D6, 2026-10-23): xfig's 30 figure libraries (2,540 figures; Charts, house_plans and transit left out for their licences) in `libraries/` with an ASSETS.md row each (`cargo xtask assets` checks them); `crates/engine/src/library.rs` (scan, read, place; ports `w_library.c`, `e_placelib.c`), commands `library.{panel,list,objects,hand,place,transform,save}` in `cmd/library.rs`, `ToolState::Library` in `tools/mod.rs` (xfig's buttons and keys r l h v s S, hints), the panel `crates/ui-egui/src/library.rs` (thumbnails from the canvas renderer, Icon/List view, icon size, comments, Copy to library); tests `crates/engine/tests/library.rs`, `library_panel_places_an_object` in `ui_smoke.rs`.
- Verified on macOS by driving the app over the control channel and reviewing screenshots.

## Revision history

| Date | Overall | Note |
|------|--------:|------|
| 2026-10-23 | ≈84 % | D10: copy counts (Attributes 87 → 90 %, 6 → 3 h). |
| 2026-10-23 | ≈84 % | Recount after D6 (Libraries 0 → 85 %, Drawing modes 90 → 95 %).
| 2026-10-23 | ≈79 % | Recount after B5, C1, C2, P2 (Printing 10 → 85 %, Platforms 50 → 70 %).
| 2026-10-23 | ≈73 % | P2: platform behaviour checks (paths per OS, non-ASCII names, HiDPI, command line); Platforms 65 → 70 %. |
| 2026-10-23 | ≈73 % | C2: browser file I/O (open by picker and drop, save and export as downloads); Platforms 55 → 65 %. |
| 2026-10-23 | ≈73 % | C1: web packaging with trunk (`cargo xtask web`); Platforms 50 → 55 %. |
| 2026-10-23 | ≈79 % | D6: library mode (bundled libraries, panel with thumbnails, placing, copy to library); Libraries 0 → 85 %, Drawing modes 90 → 95 %, 40 h less. |
| 2026-10-23 | ≈73 % | D10, W10-13, W10-14, W9 fix: fractions of an inch and the dimension line preview; saved versions; cap/join pictures. Scores unchanged. |
| 2026-10-22 | ≈73 % | W10-23, -24, -31, -32, -33, W10-3: Global settings rows with effect, Unit/Scale panel, Arrow Size panel, number pop-ups, seven align and nine grid choices; Settings & styles 75 → 80 %. |
| 2026-10-22 | ≈73 % | Recount: Text 55 → 65 % (E10), Settings & styles 15 → 75 % (E1 persistence, recent files, E6 styles, E4 colours); the table had lagged behind. |
| 2026-10-22 | ≈73 % | E5, E8: character map (Symbol and Dingbats stored as the font codes) and shortcut sheet; Text 65 → 70 %, 8 h less; headline not recounted. |
| 2026-10-22 | ≈65 % | E3, S4-6: zoom box and shape factor on the canvas; View & navigation 96 → 100 % (rounds to the same overall). |
| 2026-10-22 | ≈65 % | S4-3, S4-8, S4-9: Digitize (export line as data), relative picture names on Save As, Autorefresh mode, New window; evidence only, score unchanged. |
| 2026-10-23 | ≈73 % | E11, E9: font chooser (`ui-egui/src/font_chooser.rs`, names drawn through `rustfig_text::shape`, search, size, preview; tests `font_chooser_*` in `ui_smoke.rs`), theme and UI scale settings (`app.theme`, `app.ui_scale`, `RustFigPrefs::theme/ui_scale`; test `theme_and_scale_persist`); evidence only, score unchanged. |
| 2026-10-22 | ≈65 % | W10-28, W10-2: bundled manual (27 topics, search, Help > RustFig Manual, `help.open`); evidence only, score unchanged. |
| 2026-10-22 | ≈65 % | D7: Angle Geom (Attributes 80 → 83 %). |
| 2026-10-22 | ≈65 % | D14: smart links extended (Attributes 83 → 84 %). |
| 2026-10-22 | ≈65 % | D8: linked move (Attributes 77 → 80 %, 10 h less). |
| 2026-10-22 | ≈65 % | D5: measure modes (angle, length, area), Editing 80 → 81 %. |
| 2026-10-22 | ≈65 % | D4: Tangent/normal mode (`object.tangent_line`, `tool.tangent`, Tangent/Norm Length), Editing 79 → 80 %. |
| 2026-10-22 | ≈65 % | D15: Chop extended (boxes, ellipses, splines, compounds; Trim, Split, Cut all, Extend), Editing 78 → 79 %. |
| 2026-10-22 | ≈65 % | D3: Chop mode (cut objects at intersections), Editing 75 → 78 %. |
| 2026-10-21 | ≈65 % | W16, W17: one mouse scheme and one layout; save query for open compounds; no score moved. |
| 2026-10-21 | ≈65 % | DD9: evidence paths after the dedup series (reader, bounds, exports through fig2dev); no score moved. |
| 2026-10-18 | ≈65 % | B1, B3, B9: TikZ, PSTricks, EMF, Tk/Perl-Tk ported (Export 57 → 66 %, 50 h less). |
| 2026-10-17 | ≈64 % | B3: pict2e ported, byte-identical to fig2dev on 285 of 300 sample figures (Export 55 → 57 %, 14 h less). |
| 2026-10-16 | ≈64 % | B12: fig2dev self-contained, every export through `convert` (Export stays 55 %: architecture, no new language). |
| 2026-10-16 | ≈64 % | G1, B2, B9 audit passes: old Fig files read (File format 85 → 92 %); pstex/pdftex and every ported fig2dev language exported from the app (Export 40 → 55 %; hours up with the audit's B11). |
| 2026-10-21 | — | W10-1: Update Control (update flags per attribute button; evidence only, score unchanged). |
| 2026-10-21 | — | W19: Attributes panel tightened, xfig's layout kept (look only; score unchanged). |
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
