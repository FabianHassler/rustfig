# Function-level source audit of xfig (S4)

**Status:** 2026-10-22 (D5: the `e_measure.c` functions, the measure results and the angle `compute_*` functions are ported). 2026-10-22 (D4: the `e_tangent.c` functions, the Tangent/Norm Length button and `smart_next_{arc,ellipse,line,spline}_found` are ported). 2026-10-22 (S4 done: every function definition of `old/xfig-3.2.9a/src/*.c` has a status). Regenerate the table with `cargo xtask source-audit` (skips quietly when `old/` is absent; keeps the status, note and task columns of existing rows, merged by file, function name and occurrence).

S0 ([`source-map.md`](source-map.md)) maps C files to RustFig modules; this file goes one level down, to every function. It covers xfig only. fig2dev's functions were audited one by one in the fig2dev repository (<https://github.com/FabianHassler/fig2dev>, its audit passes; the inventory of the first rounds is `plan/fig2dev-audit.md` here, local and gitignored) and their hardening is task B11; they are not repeated here.

## How the audit was done

1. `cargo xtask source-audit` (`xtask/src/audit.rs`) strips comments, strings and preprocessor lines from each C file and lists every function definition (name, file, line) with a column-0 header at brace depth 0, K&R parameter lists included. It was checked against a second count of column-0 `{` lines; xfig's `xfigmain.c` is a generated copy of `main.c` and is listed with the same statuses.
2. Each function was assigned a status by file, against the RustFig module that does the job (names in the notes are real items: tools, commands, dialogs, crates). Where a function could hide a behaviour that RustFig lacks, the C body was read and compared with RustFig's behaviour, mouse hints and docs (this found the S4-n gaps below). "Ported" means a counterpart with the same behaviour exists, not that the code was transliterated; ported C code is marked in the Rust file and `LICENSE-xfig`.

## Statuses

- **ported**: RustFig has the function's behaviour (module and item in the note).
- **replaced**: a deliberate modern equivalent does the job, with the reason (Xt widget code → egui, `~/.xfigrc` → `settings.toml`, ghostscript calls → fig2dev's port, X11 drawing → the renderer, ...).
- **not needed**: X11/Xt plumbing, memory management, platform workarounds. Where the function is visible to users the note starts with **PROPOSAL** (see below).
- **partial**: part of the behaviour exists; the note says what is missing and the task adds it. (An addition to the four statuses the task named; a function that is half there would otherwise be hidden under "ported" or inflate "missing".)
- **missing**: no counterpart; the Task column names the plan task that adds it. Where a modern replacement is planned but not built (printing, B5), the status is **missing**, not replaced.

## Summary

<!-- audit-counts -->

| Status | Functions |
|---|---:|
| ported | 868 |
| replaced | 584 |
| not needed | 218 |
| partial | 58 |
| missing | 384 |
| **total** | **2112** (plus 14 in `xfigmain.c`, a generated copy of `main.c`) |

Missing and partial functions by task (a function counts under each task it names):

- **D10** (67): `d_line.c`: `init_trace_drawing`; `e_align.c`: `init_align_canvas`; `e_compound.c`: `close_compound`; `e_flip.c`: `set_unset_anchor`; `e_rotate.c`: `set_unset_center`; `e_scale.c`: `rescale_dimension_line`; `f_util.c`: `dimline_components`; `u_create.c`: `create_dimension_line`, `create_tick`, `create_dimline_ticks`, `backward_dim_arrow`, `forward_dim_arrow`; `u_drag.c`: `array_place_ellipse`, `array_place_arc`, `array_place_line`, `array_place_text`, `array_place_spline`, `array_place_compound`; `w_canvas.c`: `canvas_selected`, `toggle_show_lengths`; `w_indpanel.c`: `popup_dimline_panel`, `line_color_select`, `box_color_select`, `text_color_select`, `set_fixed_text`, `set_actual_text`, `draw_cur_dimline`, `dimline_style_select`, `left_arrow_type_select`, `right_arrow_type_select`, `dimline_font_popup`, `dimline_set_font_image`, `dimline_panel_preview`, `dimline_panel_ok`, `dimline_panel_cancel`, `get_dimline_values`, `inc_dimline`, `dec_dimline`, `show_dimline`, `inc_numcopies`, `dec_numcopies`, `show_numcopies`, `inc_numxcopies`, `dec_numxcopies`, `show_numxcopies`, `inc_numycopies`, `dec_numycopies`, `show_numycopies`; `w_keyboard.c`: `next_keyboard_history`, `prior_keyboard_history`, `ignore_keyboard_input`, `handle_keyboard_input`, `create_keyboard_panel`, `popup_keyboard_panel`, `popdown_keyboard_panel`; `w_mousefun.c`: `draw_mousefun_kbd`, `clear_mousefun_kbd`, `init_kbd_actions`; `w_msgpanel.c`: `boxsize_msg`, `boxsize_scale_msg`, `erase_box_lengths`, `erase_lengths`, `length_msg`, `altlength_msg`, `length_msg2`, `arc_msg`, `make_dimension_string`
- **D6** (43): `e_placelib.c`: `put_selected`, `transform_lib_obj`, `sel_place_lib_obj_proc`, `sel_place_lib_obj`, `put_draw`, `change_draw_mode`, `place_lib_object_orig`, `place_lib_object`, `move_object`, `init_move_object`, `cancel_place_lib_obj`; `w_library.c`: `SPComp`, `LRComp`, `put_selected_request`, `library_stop`, `library_dismiss`, `library_cancel`, `load_library`, `put_object_sel`, `load_lib_obj`, `NewObjectSel`, `popup_library_panel`, `create_library_panel`, `make_library_menu`, `sel_view`, `change_icon_size`, `set_preview_name`, `set_cur_obj_name`, `erase_pixmap`, `PutLibraryEntry`, `ScanLibraryDirectory`, `MakeLibrary`, `MakeObjectLibrary`, `MakeLibraryFileList`, `libraryStatus`, `preview_libobj`, `update_preview`, `set_comments`, `sel_item_icon`, `unsel_icon`, `sel_icon`, `copy_icon_to_preview`; `w_modepanel.c`: `stub_popup_library`
- **E4** (37): `w_color.c`: `create_color_panel`, `pen_fill_activate`, `restore_mixed_colors`, `set_mixed_color`, `set_user_color`, `pick_contrast`, `set_mixed_name`, `cancel_color_popup`, `del_color`, `undel_color`, `switch_edit`, `set_slider_sensitivity`, `set_color_ok`, `_set_std_color`, `set_std_color`, `_pick_memory`, `pick_memory`, `lock_toggle`, `draw_boxed`, `erase_boxed`, `color_borders`, `WhichButton`, `update_from_triple`, `update_scrl_triple`, `update_triple`, `move_scroll`, `StoreMix_and_Mem`, `Scrolled`, `Update_HSV`, `ThumbHSV`, `Thumbed`, `move_lock`, `HSVToRGB`, `RGBToHSV`, `PctToRGB`, `DoGrabPixel`, `doGrab`
- **E6** (34): `w_style.c`: `construct_style`, `set_style`, `print_indent`, `print_style`, `print_style_family`, `print_family_set`, `get_nstyle_line`, `trim`, `parse_style`, `parse_family_style`, `parse_family_set`, `load_family_set`, `save_family_set`, `add_family_to_family_set`, `add_style_to_family`, `delete_family_from_family_set`, `delete_style_from_family`, `build_family_text`, `build_style_text`, `style_update`, `family_select`, `style_select`, `add_family`, `delete_family`, `add_style`, `delete_style`, `save_style`, `load_style`, `confirm_close_style`, `close_style`, `init_manage_style_panel`, `add_style_actions`, `setup_manage_style_panel`, `popup_manage_style_panel`
- **B5** (33): `u_print.c`: `cmdlinetoargarray`, `start_argumentlist`, `print_to_printer`, `print_to_batchfile`, `strsub`, `pass_environment`, `print_spawn_printcmd`; `w_print.c`: `print_panel_dismiss`, `do_print`, `fit_page`, `get_magnif`, `update_mag`, `update_figure_size`, `do_print_batch`, `do_clear_batch`, `update_batch_count`, `orient_select`, `just_select`, `papersize_select`, `multiple_select`, `overlap_select`, `background_select`, `print_grid_minor_select`, `print_grid_major_select`, `print_command_select`, `printer_select`, `print_update_figure_size`, `popup_print_panel`, `create_print_panel`, `switch_print_layers`, `switch_bound`, `parse_printcap`, `make_layer_choice`
- **D8** (32): `e_movept.c`: `init_arcpointmoving`, `cancel_movedarcpoint`, `fix_movedarcpoint`, `init_splinepointmoving`, `cancel_movedsplinepoint`, `fix_movedsplinepoint`, `init_linepointmoving`, `cancel_movedlinepoint`, `fix_movedlinepoint`; `u_create.c`: `new_link`; `u_elastic.c`: `elastic_linelink`, `elastic_links`, `elastic_arclink`; `u_smartsearch.c`: `init_smart_searchproc_left`, `init_smart_searchproc_middle`, `init_smart_searchproc_right`, `smart_object_search_left`, `smart_object_search_middle`, `smart_object_search_right`, `set_smart_points`, `init_smart_search`, `do_smart_object_search`, `smart_next_arc_found`, `smart_next_ellipse_found`, `smart_next_line_found`, `smart_next_spline_found`, `smart_next_text_found`, `smart_next_compound_found`, `smart_show_objecthighlight`, `smart_erase_objecthighlight`, `smart_toggle_objecthighlight`, `smart_null_proc`
- **W10-10** (26): `w_export.c`: `exp_getxyoff`, `preview_select`, `orient_select`, `just_select`, `papersize_select`, `multiple_select`, `overlap_select`, `set_postscript_pdf_options`, `set_export_mask`, `smooth_select`, `background_select`, `transp_select`, `export_grid_minor_select`, `export_grid_major_select`, `manage_optional`, `export_update_figure_size`, `fit_page`, `get_magnif`, `update_mag`, `update_figure_size`, `get_quality`, `exp_xoff_unit_select`, `exp_yoff_unit_select`, `toggle_hpgl_pcl_switch`, `toggle_hpgl_font`, `toggle_pdf_pagemode`
- **W10-4** (26): `w_export.c`: `exp_getxyoff`, `preview_select`, `orient_select`, `just_select`, `papersize_select`, `multiple_select`, `overlap_select`, `set_postscript_pdf_options`, `set_export_mask`, `smooth_select`, `background_select`, `transp_select`, `export_grid_minor_select`, `export_grid_major_select`, `manage_optional`, `export_update_figure_size`, `fit_page`, `get_magnif`, `update_mag`, `update_figure_size`, `get_quality`, `exp_xoff_unit_select`, `exp_yoff_unit_select`, `toggle_hpgl_pcl_switch`, `toggle_hpgl_font`, `toggle_pdf_pagemode`
- **D11** (14): `w_capture.c`: `captureImage`, `rshift`, `getImageData`, `selectedRootArea`, `drawRect`, `getCurrentColors`, `canHandleCapture`; `w_digitize.c`: `popup_digitize_panel`, `create_digitize_panel`, `digitize_panel_done`, `digitize_panel_cancel`, `update_example`, `switch_file_mode`; `w_zoom.c`: `unzoom`
- **W10-16** (12): `e_edit.c`: `reread_picfile`, `expand_pic`, `shrink_pic`, `origsize_pic`, `scale_percent_pic`, `reposition_picture`, `resize_picture`, `flip_pic_select`, `rotation_select`, `image_edit_button`, `browse_button`, `what_rotation`
- **S4-1** (10): `u_markers.c`: `toggle_csrhighlight`, `toggle_ellipsehighlight`, `toggle_archighlight`, `toggle_texthighlight`, `toggle_compoundhighlight`, `toggle_linehighlight`, `toggle_splinehighlight`; `u_search.c`: `show_objecthighlight`, `erase_objecthighlight`, `toggle_objecthighlight`
- **W10-15** (10): `e_edit.c`: `make_window_compound`, `modify_compound`, `reposition_top`, `reposition_bottom`, `rescale_compound`, `get_new_compound_values`, `done_compound`, `check_depth`, `collapse_depth`, `collapse_depths`
- **D4** (9): `e_tangent.c`: `tangent_selected`, `init_tangent_adding`, `init_normal_adding`, `tangent_or_normal`, `tangent_normal_line`; `w_indpanel.c`: `inc_tangnormlen`, `dec_tangnormlen`, `show_tangnormlen`; `w_modepanel.c`: `stub_tangent_selected`
- **E10** (9): `d_text.c`: `new_text_down`, `new_text_up`, `split_at_cursor`, `initialize_char_handler`, `terminate_char_handler`, `char_handler`; `w_canvas.c`: `get_canvas_clipboard`, `paste_primary_selection`, `canvas_paste`
- **W10-2** (8): `w_help.c`: `launch_refman`, `launch_refpdf_en`, `launch_refpdf_jp`, `launch_howto`, `launch_man`, `launch_viewer`, `check_docfile`, `help_ok`
- **W10-31** (8): `w_indpanel.c`: `arrowsize_panel_dismiss`, `arrowsize_panel_cancel`, `arrowsize_panel_set`, `set_arrow_size_state`, `popup_arrowsize_panel`, `inc_arrowsize`, `dec_arrowsize`, `show_arrowsize`
- **E3** (7): `w_canvas.c`: `canvas_selected`; `w_zoom.c`: `zoom_selected`, `my_box`, `elastic_mybox`, `init_zoombox_drawing`, `do_zoom`, `cancel_zoom`
- **F5** (7): `w_srchrepl.c`: `popup_spell_check_panel`, `spell_panel_dismiss`, `spell_check`, `write_text_from_compound`, `spell_select_word`, `spell_correct_word`, `show_spell_msg`
- **S4-2** (7): `w_cmdpanel.c`: `paste`, `cancel_paste`, `paste_draw`, `move_paste_object`, `init_move_paste_object`, `place_object`, `place_object_orig_posn`
- **W10-12** (7): `w_file.c`: `file_getxyoff`, `file_xoff_unit_select`, `file_yoff_unit_select`, `check_cancel`, `clear_preview`, `file_preview_stop`, `preview_figure`
- **E5** (6): `w_cmdpanel.c`: `charmap_font_dimensions`, `refresh_character_panel`, `character_panel_close`, `popup_character_map`, `paste_character`, `redraw_character`
- **S4-3** (5): `f_picobj.c`: `internal_path`, `xf_relpath`, `relative_path`, `xf_external_path`, `external_path`
- **S4-4** (5): `w_cursor.c`: `init_cursor`, `recolor_cursors`, `reset_cursor`, `set_temp_cursor`, `set_cursor`
- **W10-32** (5): `w_indpanel.c`: `nval_panel_dismiss`, `toggle_int_zoom`, `nval_panel_cancel`, `nval_panel_set`, `popup_nval_panel`
- **F3** (3): `d_text.c`: `initialize_char_handler`, `terminate_char_handler`, `char_handler`
- **S3** (2): `f_util.c`: `update_fig_files`; `main.c`: `main`
- **S4-5** (2): `e_convert.c`: `init_convert_line_spline`, `box_2_box`
- **S4-6** (2): `e_edit.c`: `change_sfactor`; `w_canvas.c`: `canvas_selected`
- **S4-7** (2): `w_mousefun.c`: `draw_shift_mousefun_canvas`, `draw_shift_mousefun_canvas2`
- **W10-36** (2): `w_color.c`: `DoGrabPixel`, `doGrab`
- **E9** (1): `w_canvas.c`: `canvas_selected`
- **W10-3** (1): `w_canvas.c`: `toggle_show_vertexnums`
- **W10-33** (1): `w_indpanel.c`: `show_gridmode`

<!-- /audit-counts -->

### New tasks (S4-n, in [`plan.md`](plan.md))

Functions the audit found missing with no task yet:

| Id | Gap | C functions | Hours |
|----|-----|-------------|------:|
| S4-1 | Shift-click highlights the object found and, with Shift held, cycles through the overlapping ones before one acts (xfig manual, editing: "select with Shift") | `show_/erase_/toggle_objecthighlight`, `toggle_csrhighlight`, `toggle_*highlight`, `do_object_search` | 6 |
| S4-2 | Paste attaches the objects to the mouse: left places, middle at the original position, right cancels | `paste`, `place_object`, `place_object_orig_posn`, `paste_draw`, `move_paste_object`, `cancel_paste` | 3 |
| S4-3 | Relative picture file names follow the figure when it is saved in another folder | `relative_path`, `xf_relpath`, `external_path`, `internal_path` | 2 |
| S4-4 | A cursor per mode (egui's cursor set: crosshair, move, text, grab, ...) | `set_cursor`, `init_cursor`, `reset_cursor` | 2 |
| S4-5 | Convert: box ↔ arc-box with the left button; remember the spline kind round trip | `box_2_box`, `init_convert_line_spline` | 1 |
| S4-6 | Ctrl+Shift+click on a spline point in an edit mode changes its shape factor (left more approximating, middle cycle, right more interpolating) | `change_sfactor` | 3 |
| S4-7 | The mouse-function indicator shows the Shift, Ctrl and Ctrl+Shift variants while the key is held | `draw_shift_mousefun_canvas`, `draw_shift_mousefun_canvas2` | 3 |

Total 20 h. Everything else missing or partial is already a task (D1 to D13, E3 to E12, F5, W10-n, B5, S3); the lists above give the functions each task must cover. `xfig -update` (`update_fig_files`) is part of S3.

### Proposals for the user's decision

These are visible to users and proposed as **not needed**; they stay open until accepted (nothing is dropped meanwhile).

1. **Autorefresh mode** (`set_autorefresh`, `cancel_autorefresh`, `toggle_refresh_mode`, `check_refresh`): xfig redraws the canvas at an interval, and reloads a file changed on disk, to repair X11 drawing leftovers. egui redraws every frame. Proposal: drop it (W10-3 already says "probably not needed"); optionally offer "reload when the file changed on disk" later.
2. **New xfig...** in the file panel (`new_xfig_request`): starts a second xfig process. RustFig has one window per process; the OS opens a second instance (already recorded in window-parity.md as low value).
3. **Splash screen** (`splash_screen`, `clear_splash`): the About box carries the credits.
4. **Terminal bell** (`beep`, 83 calls): errors reach the message panel; no sound.
5. **Print batch list and printcap parsing** (`do_print_batch`, `do_clear_batch`, `update_batch_count`, `print_to_batchfile`, `parse_printcap`): not carried into the modern print dialog (B5): the OS print dialog chooses the printer and queues jobs.
6. **Digitize** (6 functions of `w_digitize.c`, a tablet pen against a scanned figure): D11 keeps it as xfig's function; proposal: replace by clicking on a picture (pictures are already placed with coordinates) or drop it, tablets being rare. **Capture** (`w_capture.c`): through the OS screenshot facilities and E7's image paste.

## Revision history

| Date | Note |
|------|------|
| 2026-10-21 | First version (S4): 2,126 function definitions of xfig 3.2.9a listed by `cargo xtask source-audit`, each with a status; S4-1 to S4-7 added; `source-map.md` statuses corrected. |
| 2026-10-22 | D3: `e_chop.c`'s 14 functions and `w_modepanel.c`'s `stub_chop_selected` marked ported. |

## Table

<!-- audit-table -->

| File | Function | Line | Status | RustFig item or reason | Task |
|---|---|---:|---|---|---|
| `d_arc.c` | `arc_drawing_selected` | 73 | ported | tools: `ToolKind::Arc` (3 clicks, `points_event`, `arc_from_center` for centre mode) |  |
| `d_arc.c` | `init_arc_drawing` | 87 | ported | tools: `ToolKind::Arc` (3 clicks, `points_event`, `arc_from_center` for centre mode) |  |
| `d_arc.c` | `init_arc_c_drawing` | 128 | ported | tools: `ToolKind::Arc` (3 clicks, `points_event`, `arc_from_center` for centre mode) |  |
| `d_arc.c` | `cancel_arc` | 144 | ported | tools: `ToolKind::Arc` (3 clicks, `points_event`, `arc_from_center` for centre mode) |  |
| `d_arc.c` | `get_arcpoint` | 177 | ported | tools: `ToolKind::Arc` (3 clicks, `points_event`, `arc_from_center` for centre mode) |  |
| `d_arc.c` | `create_arcobject` | 204 | ported | tools: `ToolKind::Arc` (3 clicks, `points_event`, `arc_from_center` for centre mode) |  |
| `d_arc.c` | `resizing_arc` | 305 | replaced | rubber band drawn from `Engine::overlay()` |  |
| `d_arc.c` | `elastic_arc` | 325 | replaced | rubber band drawn from `Engine::overlay()` |  |
| `d_arcbox.c` | `arcbox_drawing_selected` | 43 | ported | tools: `ToolKind::ArcBox` (`drag_event`, `finish_drag`) |  |
| `d_arcbox.c` | `init_arc_box_drawing` | 56 | ported | tools: `ToolKind::ArcBox` (`drag_event`, `finish_drag`) |  |
| `d_arcbox.c` | `cancel_arc_boxobject` | 72 | ported | tools: `ToolKind::ArcBox` (`drag_event`, `finish_drag`) |  |
| `d_arcbox.c` | `create_arc_boxobject` | 80 | ported | tools: `ToolKind::ArcBox` (`drag_event`, `finish_drag`) |  |
| `d_box.c` | `box_drawing_selected` | 43 | ported | tools: `ToolKind::Box` (`drag_event`, `finish_drag`) |  |
| `d_box.c` | `init_box_drawing` | 56 | ported | tools: `ToolKind::Box` (`drag_event`, `finish_drag`) |  |
| `d_box.c` | `cancel_box` | 72 | ported | tools: `ToolKind::Box` (`drag_event`, `finish_drag`) |  |
| `d_box.c` | `create_boxobject` | 82 | ported | tools: `ToolKind::Box` (`drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `circle_ellipse_byradius_drawing_selected` | 55 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `init_ellipsebyradius_drawing` | 68 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `cancel_ellipsebyrad` | 87 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `create_ellipsebyrad` | 96 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `circle_ellipse_bydiameter_drawing_selected` | 133 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `init_ellipsebydiameter_drawing` | 146 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `cancel_ellipsebydia` | 165 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `create_ellipsebydia` | 174 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `init_circlebyradius_drawing` | 214 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `cancel_circlebyrad` | 232 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `create_circlebyrad` | 241 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `init_circlebydiameter_drawing` | 281 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `cancel_circlebydia` | 299 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_ellipse.c` | `create_circlebydia` | 308 | ported | tools: circle/ellipse by radius and diameter (`ToolKind::{CircleRadius,CircleDiameter,EllipseRadii,EllipseDiameter}`, `drag_event`, `finish_drag`) |  |
| `d_line.c` | `line_drawing_selected` | 54 | ported | tools: polyline, polygon, freehand (`points_event`, `push_point`, `finish_points`) |  |
| `d_line.c` | `init_line_freehand_drawing` | 76 | ported | tools: polyline, polygon, freehand (`points_event`, `push_point`, `finish_points`) |  |
| `d_line.c` | `init_line_drawing` | 85 | ported | tools: polyline, polygon, freehand (`points_event`, `push_point`, `finish_points`) |  |
| `d_line.c` | `cancel_line_drawing` | 98 | ported | tools: polyline, polygon, freehand (`points_event`, `push_point`, `finish_points`) |  |
| `d_line.c` | `init_trace_drawing` | 114 | partial | polyline, freehand and the latex/manhattan/mountain constraints ported (D7); dimension-line branch (two-point line with ticks) is not | D10 |
| `d_line.c` | `freehand_get_intermediatepoint` | 163 | ported | tools: polyline, polygon, freehand (`points_event`, `push_point`, `finish_points`) |  |
| `d_line.c` | `get_intermediatepoint` | 175 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `d_line.c` | `create_lineobject` | 236 | ported | tools: polyline, polygon, freehand (`points_event`, `push_point`, `finish_points`) |  |
| `d_picobj.c` | `picobj_drawing_selected` | 48 | ported | tools: `ToolKind::Picture` (box drag, then picture file through `engine::import::load_picture`) |  |
| `d_picobj.c` | `init_picobj_drawing` | 61 | ported | tools: `ToolKind::Picture` (box drag, then picture file through `engine::import::load_picture`) |  |
| `d_picobj.c` | `cancel_picobj` | 69 | ported | tools: `ToolKind::Picture` (box drag, then picture file through `engine::import::load_picture`) |  |
| `d_picobj.c` | `create_picobj` | 77 | ported | tools: `ToolKind::Picture` (box drag, then picture file through `engine::import::load_picture`) |  |
| `d_regpoly.c` | `regpoly_drawing_selected` | 47 | ported | tools: `ToolKind::RegularPolygon` (`geom::regular_polygon`, number of sides from the indicator) |  |
| `d_regpoly.c` | `init_regpoly_drawing` | 60 | ported | tools: `ToolKind::RegularPolygon` (`geom::regular_polygon`, number of sides from the indicator) |  |
| `d_regpoly.c` | `cancel_regpoly` | 77 | ported | tools: `ToolKind::RegularPolygon` (`geom::regular_polygon`, number of sides from the indicator) |  |
| `d_regpoly.c` | `create_regpoly` | 87 | ported | tools: `ToolKind::RegularPolygon` (`geom::regular_polygon`, number of sides from the indicator) |  |
| `d_spline.c` | `spline_drawing_selected` | 51 | ported | tools: the six spline modes (`spline_kind`, `points_event`, `points_object`) |  |
| `d_spline.c` | `init_spline_freehand_drawing` | 64 | ported | tools: the six spline modes (`spline_kind`, `points_event`, `points_object`) |  |
| `d_spline.c` | `init_spline_drawing` | 71 | ported | tools: the six spline modes (`spline_kind`, `points_event`, `points_object`) |  |
| `d_spline.c` | `init_spline_drawing2` | 78 | ported | tools: the six spline modes (`spline_kind`, `points_event`, `points_object`) |  |
| `d_spline.c` | `create_splineobject` | 96 | ported | tools: the six spline modes (`spline_kind`, `points_event`, `points_object`) |  |
| `d_spline.c` | `make_sfactors` | 169 | ported | X-spline shape factors set when the object is made (`points_object`, model `Spline::factors`) |  |
| `d_subspline.c` | `create_subspline` | 44 | replaced | the overlay evaluates the whole curve while drawing; no sub-spline list |  |
| `d_subspline.c` | `free_subspline` | 67 | not needed | memory management |  |
| `d_subspline.c` | `draw_subspline` | 74 | replaced | the overlay evaluates the whole curve while drawing; no sub-spline list |  |
| `d_subspline.c` | `add_subspline_point` | 84 | replaced | the overlay evaluates the whole curve while drawing; no sub-spline list |  |
| `d_subspline.c` | `extract_subspline` | 101 | replaced | the overlay evaluates the whole curve while drawing; no sub-spline list |  |
| `d_text.c` | `text_drawing_selected` | 160 | ported | tools: `ToolKind::Text` (`text_event`, `finish_text`) |  |
| `d_text.c` | `commit_current_text` | 188 | ported | tools: `ToolKind::Text` (`text_event`, `finish_text`) |  |
| `d_text.c` | `finish_n_start` | 208 | ported | tools: `ToolKind::Text` (`text_event`, `finish_text`) |  |
| `d_text.c` | `finish_text_input` | 221 | ported | tools: `ToolKind::Text` (`text_event`, `finish_text`) |  |
| `d_text.c` | `cancel_text_input` | 241 | ported | tools: `ToolKind::Text` (`text_event`, `finish_text`) |  |
| `d_text.c` | `new_text_line` | 276 | ported | Enter finishes the line and starts the next text below (`text_event`, text step) |  |
| `d_text.c` | `new_text_down` | 303 | missing | Up/Down arrows moving to the previous/next text line | E10 |
| `d_text.c` | `new_text_up` | 329 | missing | Up/Down arrows moving to the previous/next text line | E10 |
| `d_text.c` | `set_ascentdescent` | 356 | ported | tools: `ToolKind::Text` (`text_event`, `finish_text`) |  |
| `d_text.c` | `overlay_text_input` | 391 | ported | tools: `ToolKind::Text` (`text_event`, `finish_text`) |  |
| `d_text.c` | `init_text_input` | 450 | ported | tools: `ToolKind::Text` (`text_event`, `finish_text`) |  |
| `d_text.c` | `new_text` | 583 | ported | tools: `ToolKind::Text` (`text_event`, `finish_text`) |  |
| `d_text.c` | `split_at_cursor` | 618 | missing | placing the caret inside an existing text by clicking at a character (RustFig puts the caret at the end) | E10 |
| `d_text.c` | `begin_utf8char` | 727 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `end_utf8char` | 740 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `draw_cursor` | 776 | replaced | the caret is drawn by the canvas from the overlay text; egui repaints, so no blink timer |  |
| `d_text.c` | `initialize_char_handler` | 786 | partial | Left/Right/Home/End/Backspace/Delete ported; the Ctrl-key editing (kill line, word, begin/end) and paste into the text are not | E10, F3 |
| `d_text.c` | `terminate_char_handler` | 799 | partial | Left/Right/Home/End/Backspace/Delete ported; the Ctrl-key editing (kill line, word, begin/end) and paste into the text are not | E10, F3 |
| `d_text.c` | `char_handler` | 807 | partial | Left/Right/Home/End/Backspace/Delete ported; the Ctrl-key editing (kill line, word, begin/end) and paste into the text are not | E10, F3 |
| `d_text.c` | `turn_on_blinking_cursor` | 1109 | replaced | the caret is drawn by the canvas from the overlay text; egui repaints, so no blink timer |  |
| `d_text.c` | `turn_off_blinking_cursor` | 1128 | replaced | the caret is drawn by the canvas from the overlay text; egui repaints, so no blink timer |  |
| `d_text.c` | `blink` | 1136 | replaced | the caret is drawn by the canvas from the overlay text; egui repaints, so no blink timer |  |
| `d_text.c` | `move_blinking_cursor` | 1165 | replaced | the caret is drawn by the canvas from the overlay text; egui repaints, so no blink timer |  |
| `d_text.c` | `GetPreferredGeomerty` | 1186 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `SetGeometry` | 1195 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `xim_set_ic_geometry` | 1204 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `xim_initialize` | 1248 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `xim_set_spot` | 1358 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `is_preedit_running` | 1383 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `kill_preedit` | 1395 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `close_preedit_proc` | 1404 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `open_preedit_proc` | 1418 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `d_text.c` | `paste_preedit_proc` | 1454 | replaced | UTF-8 composition and input methods come from egui/winit (IME); compose key is `rustfig_text::compose` |  |
| `e_addpt.c` | `point_adding_selected` | 50 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `init_point_adding` | 68 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `wrapup_pointadding` | 113 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `cancel_pointadding` | 121 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `cancel_line_pointadding` | 131 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `init_splinepointadding` | 145 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `fix_splinepoint_adding` | 167 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `splinepoint_adding` | 202 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `init_linepointadding` | 257 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `fix_linepoint_adding` | 282 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `linepoint_adding` | 309 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_addpt.c` | `find_endpoints` | 344 | ported | tools: `ToolKind::AddPoint` (`point_event`, `find_segment`, `preview_point`), command `point.add` |  |
| `e_align.c` | `align_selected` | 69 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `init_align_canvas` | 86 | missing | Align to canvas (alignment of the objects to the page, Align mode with the canvas button) | D10 |
| `e_align.c` | `init_align` | 142 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `align_ellipse` | 190 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `align_arc` | 204 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `align_line` | 218 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `align_spline` | 232 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `align_compound` | 246 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `align_text` | 260 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `get_dx_dy` | 274 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `pos_arc` | 328 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `pos_ellipse` | 365 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `pos_line` | 402 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `pos_spline` | 439 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `pos_text` | 476 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `pos_compound` | 512 | ported | `cmd/arrange.rs` `arrange.align` and the Align tool (`ToolKind::Align`) |  |
| `e_align.c` | `init_distrib_centres` | 563 | ported | `arrange.distribute` (centres, edges); the Align mode does not offer them yet | W10-33 |
| `e_align.c` | `init_distrib_edges` | 652 | ported | `arrange.distribute` (centres, edges); the Align mode does not offer them yet | W10-33 |
| `e_align.c` | `adjust_object_pos` | 761 | ported | `arrange.distribute` (centres, edges); the Align mode does not offer them yet | W10-33 |
| `e_align.c` | `distribute_horizontally` | 809 | ported | `arrange.distribute` (centres, edges); the Align mode does not offer them yet | W10-33 |
| `e_align.c` | `distribute_vertically` | 886 | ported | `arrange.distribute` (centres, edges); the Align mode does not offer them yet | W10-33 |
| `e_arrow.c` | `arrow_head_selected` | 50 | ported | tools: `ToolKind::Arrow`, commands `object.toggle_forward_arrow`, `object.toggle_backward_arrow` |  |
| `e_arrow.c` | `add_arrow_head` | 66 | ported | tools: `ToolKind::Arrow`, commands `object.toggle_forward_arrow`, `object.toggle_backward_arrow` |  |
| `e_arrow.c` | `delete_arrow_head` | 89 | ported | tools: `ToolKind::Arrow`, commands `object.toggle_forward_arrow`, `object.toggle_backward_arrow` |  |
| `e_arrow.c` | `add_linearrow` | 112 | ported | tools: `ToolKind::Arrow`, commands `object.toggle_forward_arrow`, `object.toggle_backward_arrow` |  |
| `e_arrow.c` | `add_arcarrow` | 138 | ported | tools: `ToolKind::Arrow`, commands `object.toggle_forward_arrow`, `object.toggle_backward_arrow` |  |
| `e_arrow.c` | `add_splinearrow` | 164 | ported | tools: `ToolKind::Arrow`, commands `object.toggle_forward_arrow`, `object.toggle_backward_arrow` |  |
| `e_arrow.c` | `delete_linearrow` | 186 | ported | tools: `ToolKind::Arrow`, commands `object.toggle_forward_arrow`, `object.toggle_backward_arrow` |  |
| `e_arrow.c` | `delete_arcarrow` | 222 | ported | tools: `ToolKind::Arrow`, commands `object.toggle_forward_arrow`, `object.toggle_backward_arrow` |  |
| `e_arrow.c` | `delete_splinearrow` | 256 | ported | tools: `ToolKind::Arrow`, commands `object.toggle_forward_arrow`, `object.toggle_backward_arrow` |  |
| `e_break.c` | `break_selected` | 43 | ported | tools: `ToolKind::Break`, command `object.ungroup` |  |
| `e_break.c` | `init_break_only` | 59 | ported | tools: `ToolKind::Break`, command `object.ungroup` |  |
| `e_break.c` | `init_break_tag` | 65 | ported | tools: `ToolKind::Break`, command `object.ungroup` |  |
| `e_break.c` | `init_break` | 71 | ported | tools: `ToolKind::Break`, command `object.ungroup` |  |
| `e_chop.c` | `chop_selected` | 85 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `select_axe_object` | 110 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `clear_axe_objects` | 134 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `point_sort_fcn` | 143 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `point_sort_reverse_fcn` | 152 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `create_new_line` | 162 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `check_poly` | 191 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `dice_polygons` | 228 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `dice_polylines` | 335 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `chop_polyline` | 402 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `chop_arc` | 549 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `chop_ellipse` | 699 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `select_log_object` | 829 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_chop.c` | `init_chop_right` | 883 | ported | Chop mode (cut objects at the intersection with an axe object) | D3 |
| `e_compound.c` | `init_open_compound` | 69 | ported | tools: `open_compound_event` (left button), command `compound.open` | D1 |
| `e_compound.c` | `init_open_compound_vis` | 82 | ported | tools: `open_compound_event` (middle button), `compound.open` with `keep_visible` | D1 |
| `e_compound.c` | `open_this_compound` | 97 | ported | model: `Document::open_compound` (the members become the drawing, the rest waits in `Document::open`); command `compound.open`; undo is the history, as xfig's `F_ENTER_COMP` | D1 |
| `e_compound.c` | `open_compound_selected` | 122 | ported | tools: `ToolKind::OpenCompound` (mask `M_COMPOUND`, hints `open compound` / `open, keep visible`) | D1 |
| `e_compound.c` | `close_compound` | 142 | partial | model: `Document::close_compound`; command `compound.close` (refused while drawing, selects the mode again, drops an emptied compound); the dimension-line recomputation of `rescale_dimension_line` waits for D10 | D10 |
| `e_compound.c` | `close_all_compounds` | 190 | ported | command `compound.close_all`; save, export, new and delete-all close in a copy or first (`Engine::whole_doc`) | D1 |
| `e_compound.c` | `popup_close_compound` | 233 | ported | ui-egui: `panels::close_compound_popup` (Close This Compound, Close All Compounds) | D1 |
| `e_convert.c` | `convert_selected` | 52 | ported | tools: `ToolKind::Convert` (left: `object.convert`, right: `object.toggle_closed`) |  |
| `e_convert.c` | `init_convert_open_closed` | 68 | ported | tools: `ToolKind::Convert` (left: `object.convert`, right: `object.toggle_closed`) |  |
| `e_convert.c` | `init_convert_line_spline` | 89 | partial | box <-> arc-box conversion with the left button on a box (RustFig turns a box into a spline instead); the spline kind (approximated or interpolated) is not remembered across the round trip | S4-5 |
| `e_convert.c` | `box_2_box` | 120 | partial | box <-> arc-box conversion with the left button on a box (RustFig turns a box into a spline instead); the spline kind (approximated or interpolated) is not remembered across the round trip | S4-5 |
| `e_convert.c` | `line_spline` | 149 | ported | tools: `ToolKind::Convert` (left: `object.convert`, right: `object.toggle_closed`) |  |
| `e_convert.c` | `spline_line` | 218 | ported | tools: `ToolKind::Convert` (left: `object.convert`, right: `object.toggle_closed`) |  |
| `e_convert.c` | `toggle_polyline_polygon` | 285 | ported | tools: `ToolKind::Convert` (left: `object.convert`, right: `object.toggle_closed`) |  |
| `e_convert.c` | `toggle_open_closed_spline` | 337 | ported | tools: `ToolKind::Convert` (left: `object.convert`, right: `object.toggle_closed`) |  |
| `e_copy.c` | `copy_selected` | 52 | ported | tools: `ToolKind::Copy` (`move_event` with copy), constrained copy with the middle button |  |
| `e_copy.c` | `init_arb_copy` | 71 | ported | tools: `ToolKind::Copy` (`move_event` with copy), constrained copy with the middle button |  |
| `e_copy.c` | `init_constrained_copy` | 81 | ported | tools: `ToolKind::Copy` (`move_event` with copy), constrained copy with the middle button |  |
| `e_copy.c` | `init_copy` | 91 | ported | tools: `ToolKind::Copy` (`move_event` with copy), constrained copy with the middle button |  |
| `e_copy.c` | `init_copy_to_scrap` | 138 | ported | `edit.copy` (copy to the cut buffer; system clipboard E7) |  |
| `e_delete.c` | `delete_selected` | 62 | ported | tools: `ToolKind::Delete` (`delete_event`), `edit.delete`, `edit.delete_all` |  |
| `e_delete.c` | `init_delete` | 79 | ported | tools: `ToolKind::Delete` (`delete_event`), `edit.delete`, `edit.delete_all` |  |
| `e_delete.c` | `init_delete_region` | 120 | ported | delete region with the rubber-band box (`finish_band`, `Band`) |  |
| `e_delete.c` | `cancel_delete_region` | 131 | ported | delete region with the rubber-band box (`finish_band`, `Band`) |  |
| `e_delete.c` | `delete_region` | 141 | ported | delete region with the rubber-band box (`finish_band`, `Band`) |  |
| `e_delete.c` | `init_delete_to_scrap` | 179 | ported | `edit.cut` |  |
| `e_delete.c` | `open_cut_file` | 241 | replaced | the cut buffer is in memory (and the system clipboard, E7), not a file in `~/.xfig` |  |
| `e_delete.c` | `delete_all` | 272 | ported | tools: `ToolKind::Delete` (`delete_event`), `edit.delete`, `edit.delete_all` |  |
| `e_deletept.c` | `delete_point_selected` | 45 | ported | tools: `ToolKind::DeletePoint`, command `point.delete` |  |
| `e_deletept.c` | `init_delete_point` | 60 | ported | tools: `ToolKind::DeletePoint`, command `point.delete` |  |
| `e_deletept.c` | `splinepoint_deleting` | 111 | ported | tools: `ToolKind::DeletePoint`, command `point.delete` |  |
| `e_deletept.c` | `linepoint_deleting` | 167 | ported | tools: `ToolKind::DeletePoint`, command `point.delete` |  |
| `e_edit.c` | `get_arc_type` | 568 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `put_generic_arrows` | 587 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `get_generic_arrows` | 644 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `edit_item_selected` | 685 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `edit_figure_comments` | 709 | replaced | figure comments in `dialogs::figure_comments_dialog` and `doc.set_comments`; clicking empty canvas in Edit mode to open them is D11 | D11 |
| `e_edit.c` | `popup_show_comments` | 724 | replaced | figure comments in `dialogs::figure_comments_dialog` and `doc.set_comments`; clicking empty canvas in Edit mode to open them is D11 | D11 |
| `e_edit.c` | `popdown_comments` | 815 | replaced | figure comments in `dialogs::figure_comments_dialog` and `doc.set_comments`; clicking empty canvas in Edit mode to open them is D11 | D11 |
| `e_edit.c` | `edit_item` | 821 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `reread_picfile` | 944 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `edit_spline_point` | 971 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `expand_pic` | 1001 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `shrink_pic` | 1049 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `origsize_pic` | 1098 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `scale_percent_pic` | 1132 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `make_window_figure` | 1178 | replaced | figure comments in `dialogs::figure_comments_dialog` and `doc.set_comments`; clicking empty canvas in Edit mode to open them is D11 | D11 |
| `e_edit.c` | `done_figure_comments` | 1185 | replaced | figure comments in `dialogs::figure_comments_dialog` and `doc.set_comments`; clicking empty canvas in Edit mode to open them is D11 | D11 |
| `e_edit.c` | `make_window_compound` | 1209 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `modify_compound` | 1377 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `reposition_picture` | 1390 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `resize_picture` | 1433 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `reposition_top` | 1478 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `reposition_bottom` | 1506 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `rescale_compound` | 1535 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `get_new_compound_values` | 1557 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `done_compound` | 1597 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `make_window_line` | 1685 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `get_new_line_values` | 2158 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `done_line` | 2383 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `make_window_text` | 2447 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `get_new_text_values` | 2609 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `done_text` | 2646 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `make_window_ellipse` | 2692 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `get_new_ellipse_values` | 2780 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `done_ellipse` | 2805 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `make_window_arc` | 2854 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `get_new_arc_values` | 2914 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `done_arc` | 2938 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `make_window_spline` | 2985 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `done_spline` | 3035 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `make_window_spline_point` | 3088 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `done_spline_point` | 3118 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `new_generic_values` | 3148 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `new_arrow_values` | 3218 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `done_button` | 3239 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `apply_button` | 3251 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `cancel_button` | 3264 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `edit_done` | 3276 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `edit_apply` | 3285 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `edit_cancel` | 3294 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `reset_edit_cursor` | 3303 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `generic_window` | 3316 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `spline_point_window` | 3810 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `toggle_sfactor_type` | 3931 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `change_sfactor_value` | 3945 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `scroll_sfactor_value` | 3956 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `update_sfactor_value` | 3969 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `recolor_fill_image` | 3996 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `update_fill_image` | 4006 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `set_image_pm` | 4087 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `fill_style_menu` | 4104 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `arc_type_menu` | 4220 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `join_style_panel_menu` | 4250 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `cap_style_panel_menu` | 4282 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `set_font_image` | 4324 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `font_image_panel` | 4333 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `f_menu_popup` | 4365 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `pen_color_selection_panel` | 4372 | replaced | colour menus: the colour pop-up of the indicator (E4 for the editor) | E4 |
| `e_edit.c` | `fill_color_selection_panel` | 4381 | replaced | colour menus: the colour pop-up of the indicator (E4 for the editor) | E4 |
| `e_edit.c` | `color_selection_panel` | 4390 | replaced | colour menus: the colour pop-up of the indicator (E4 for the editor) | E4 |
| `e_edit.c` | `int_panel` | 4432 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `int_panel_callb` | 4440 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `float_panel` | 4468 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `float_label` | 4500 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `int_label` | 4529 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `str_panel` | 4558 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `xy_panel` | 4661 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `f_pair_panel` | 4746 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `get_f_pos` | 4833 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `points_panel` | 4843 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `get_points` | 4981 | replaced | spline point panel `dialogs::spline_point_dialog`; shape-factor column in the point table is W10-19 | W10-19 |
| `e_edit.c` | `unit_pulldown_menu` | 4998 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `init_convert_array` | 5032 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `add_to_convert` | 5038 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `cvt_to_units` | 5046 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `cvt_to_units_str` | 5058 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `cvt_to_fig` | 5071 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `unit_select` | 5080 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `Quit` | 5116 | replaced | edit dialog `dialogs::edit_dialog` (egui) per object kind; Done/Apply/Cancel as one undo step (W10-17/18 for the live version) |  |
| `e_edit.c` | `panel_get_dim_value` | 5134 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `panel_set_scaled_int` | 5150 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `panel_clear_value` | 5158 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `arc_type_select` | 5166 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `cap_style_select` | 5202 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `join_style_select` | 5216 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `for_arrow_type_select` | 5230 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `back_arrow_type_select` | 5248 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `line_style_select` | 5266 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `pen_color_select` | 5311 | replaced | colour menus: the colour pop-up of the indicator (E4 for the editor) | E4 |
| `e_edit.c` | `fill_color_select` | 5325 | replaced | colour menus: the colour pop-up of the indicator (E4 for the editor) | E4 |
| `e_edit.c` | `color_select` | 5339 | replaced | colour menus: the colour pop-up of the indicator (E4 for the editor) | E4 |
| `e_edit.c` | `hidden_text_select` | 5375 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `rigid_text_select` | 5385 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `special_text_select` | 5395 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `textjust_select` | 5405 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `flip_pic_select` | 5415 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `rotation_select` | 5449 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `fill_style_select` | 5562 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `fill_style_sens` | 5623 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `fill_pat_sens` | 5631 | replaced | pictorial menus in the edit panel are egui combo boxes; pictures for choices are W10-21 | W10-21 |
| `e_edit.c` | `clear_text_key` | 5639 | replaced | key bindings and clipboard paste in edit panel text fields: egui text fields (platform keys) |  |
| `e_edit.c` | `paste_panel_key` | 5649 | replaced | key bindings and clipboard paste in edit panel text fields: egui text fields (platform keys) |  |
| `e_edit.c` | `get_clipboard` | 5659 | replaced | key bindings and clipboard paste in edit panel text fields: egui text fields (platform keys) |  |
| `e_edit.c` | `text_transl` | 5688 | replaced | key bindings and clipboard paste in edit panel text fields: egui text fields (platform keys) |  |
| `e_edit.c` | `change_sfactor` | 5701 | missing | Ctrl+Shift+click on a spline point in an edit mode changes its shape factor on the canvas (left: more approximating, middle: cycle shapes, right: more interpolating) | S4-6 |
| `e_edit.c` | `check_depth` | 5757 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `check_thick` | 5771 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `push_apply_button` | 5792 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `grab_button` | 5803 | replaced | Xt panel helpers: egui widgets (`xy_row`, `coord`, `arrow_rows`, drag values) |  |
| `e_edit.c` | `image_edit_button` | 5846 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `browse_button` | 5915 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_edit.c` | `collapse_depth` | 5927 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `collapse_depths` | 5941 | partial | compound panel: corner and size fields, depth range, per-text fields | W10-15 |
| `e_edit.c` | `what_rotation` | 5984 | missing | picture panel rows: file name with Reread, Browse, resize/reposition, rotation, flip, expand/shrink to original, scale by percent, Edit Image | W10-16 |
| `e_flip.c` | `flip_ud_selected` | 69 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `flip_lr_selected` | 85 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `flip_selected` | 101 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `set_unset_anchor` | 117 | missing | anchor point for flipping (set/unset with the right button) | D10 |
| `e_flip.c` | `init_flip` | 140 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `init_copynflip` | 151 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `flip_search` | 162 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `init_fliparc` | 194 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `init_flipcompound` | 216 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `init_flipellipse` | 238 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `init_flipline` | 258 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `init_flipspline` | 278 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `flip_line` | 298 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `flip_spline` | 316 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `flip_text` | 333 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `flip_ellipse` | 375 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `flip_arc` | 395 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_flip.c` | `flip_compound` | 416 | ported | commands `transform.flip_horizontal`, `transform.flip_vertical`; tools `FlipV`, `FlipH` (also copy and flip) |  |
| `e_glue.c` | `compound_selected` | 64 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `tag_object` | 80 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `init_tag_region` | 121 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `cancel_tag_region` | 132 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `tag_region` | 142 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `create_compoundobject` | 159 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `tag_obj_in_region` | 190 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `compose_compound` | 201 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `sel_ellipse` | 237 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `get_ellipse` | 258 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `sel_arc` | 284 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `get_arc` | 307 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `sel_line` | 332 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `get_line` | 362 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `sel_spline` | 387 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `get_spline` | 410 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `sel_text` | 435 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `get_text` | 453 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `sel_compound` | 478 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_glue.c` | `get_compound` | 499 | ported | tools: `ToolKind::Glue` (click or tag region, `glue_event`, `region_click`), command `object.group` |  |
| `e_joinsplit.c` | `join_split_selected` | 71 | ported | tools: `ToolKind::JoinSplit` (mask `M_POLYLINE\\\\\|M_SPLINE_O\\\\\|M_SPLINE_C`, hints `Join Lines/Splines` / `Split Line/Spline`) | D2 |
| `e_joinsplit.c` | `init_join` | 93 | ported | tools: `join_split_event` (left button: the end point under the pointer, `end_point_at`) | D2 |
| `e_joinsplit.c` | `init_split` | 113 | ported | tools: `join_split_event` (middle button), `object.split`; pictures refused | D2 |
| `e_joinsplit.c` | `join_lines` | 143 | ported | tools: `join_split_event` (state `ToolState::Join`, markers narrowed to lines); only polylines with an end picked | D2 |
| `e_joinsplit.c` | `join_splines` | 167 | ported | tools: as `join_lines`, open splines only; unlike xfig the second point is not taken for an end | D2 |
| `e_joinsplit.c` | `draw_join_marker` | 194 | ported | `Overlay::join_marker`, drawn as a magenta 20 px square | D2 |
| `e_joinsplit.c` | `erase_join_marker` | 203 | ported | `Overlay::join_marker` (nothing to erase: the overlay is redrawn) | D2 |
| `e_joinsplit.c` | `cancel_join` | 217 | ported | right button or Escape returns to the first stage | D2 |
| `e_joinsplit.c` | `join_line2` | 226 | ported | command `object.join` (`join_lines`: a polygon for both ends of one line; the same point twice does nothing) | D2 |
| `e_joinsplit.c` | `join_spline2` | 275 | ported | command `object.join` (`join_splines`: a closed spline for both ends, whose end factors take the second point's) | D2 |
| `e_joinsplit.c` | `connect_line_points` | 329 | ported | `cmd/joinsplit.rs` `join_lines`: the four end combinations; the arrows at the join go, the far arrows stay (xfig keeps the first line's) | D2 |
| `e_joinsplit.c` | `connect_spline_points` | 382 | ported | `join_splines`: points and shape factors reversed together | D2 |
| `e_joinsplit.c` | `split_line` | 460 | ported | command `object.split` (`split_polyline`: cut after the vertex or segment start; end points refused) | D2 |
| `e_joinsplit.c` | `split_polygon` | 535 | ported | `split_polyline` for polygons, boxes and arc-boxes: a polyline starting after the cut | D2 |
| `e_joinsplit.c` | `split_spline` | 571 | ported | `split_spline_at`: both pieces keep two points at least; the new ends get factor 0 | D2 |
| `e_joinsplit.c` | `split_cspline` | 668 | ported | `split_spline_at` for closed splines: opened at the cut, end factors 0 | D2 |
| `e_measure.c` | `anglemeas_selected` | 89 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `angle_msg` | 105 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `angle_save` | 111 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_anglemeas_object_m` | 125 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_anglemeas_object_r` | 132 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_anglemeas_object` | 139 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `anglemeas_line` | 162 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `anglemeas_arc` | 179 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_anglemeas_threepoints` | 193 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `anglemeas_second` | 214 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `anglemeas_third_l` | 235 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `anglemeas_third_m` | 242 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `anglemeas_third` | 249 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `cancel_anglemeas` | 275 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `freehand_line_nomsg` | 290 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `lenmeas_selected` | 302 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_lenmeas_object` | 319 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_lenmeas_object_l` | 386 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_lenmeas_object_m` | 393 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `clear_lenmeas_memory` | 400 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `areameas_selected` | 410 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_areameas_object` | 428 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_areameas_object_l` | 490 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `init_areameas_object_m` | 497 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_measure.c` | `clear_areameas_memory` | 504 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `e_move.c` | `move_selected` | 42 | ported | tools: `ToolKind::Move` (`move_event`, `commit_move`, constrained with middle button), `transform.move` |  |
| `e_move.c` | `init_arb_move` | 58 | ported | tools: `ToolKind::Move` (`move_event`, `commit_move`, constrained with middle button), `transform.move` |  |
| `e_move.c` | `init_constrained_move` | 68 | ported | tools: `ToolKind::Move` (`move_event`, `commit_move`, constrained with middle button), `transform.move` |  |
| `e_move.c` | `init_move` | 79 | ported | tools: `ToolKind::Move` (`move_event`, `commit_move`, constrained with middle button), `transform.move` |  |
| `e_movept.c` | `move_point_selected` | 80 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `init_arb_move_point` | 97 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `init_stretch_move_point` | 108 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `init_move_point` | 120 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `wrapup_movepoint` | 171 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `init_ellipsepointmoving` | 181 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `cancel_movedellipsepoint` | 246 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `fix_movedellipsepoint` | 272 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `relocate_ellipsepoint` | 303 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `init_arcpointmoving` | 358 | ported | moving a point works; a box corner resizes the attached lines' ends (`point.move`, `engine/links.rs`: D14; xfig does not link here) | |
| `e_movept.c` | `cancel_movedarcpoint` | 376 | ported | moving a point works; a box corner resizes the attached lines' ends (`point.move`, `engine/links.rs`: D14; xfig does not link here) | |
| `e_movept.c` | `fix_movedarcpoint` | 388 | ported | moving a point works; a box corner resizes the attached lines' ends (`point.move`, `engine/links.rs`: D14; xfig does not link here) | |
| `e_movept.c` | `relocate_arcpoint` | 409 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `init_splinepointmoving` | 432 | ported | moving a point works; a box corner resizes the attached lines' ends (`point.move`, `engine/links.rs`: D14; xfig does not link here) | |
| `e_movept.c` | `cancel_movedsplinepoint` | 489 | ported | moving a point works; a box corner resizes the attached lines' ends (`point.move`, `engine/links.rs`: D14; xfig does not link here) | |
| `e_movept.c` | `fix_movedsplinepoint` | 501 | ported | moving a point works; a box corner resizes the attached lines' ends (`point.move`, `engine/links.rs`: D14; xfig does not link here) | |
| `e_movept.c` | `relocate_splinepoint` | 524 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `init_compoundpointmoving` | 536 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `cancel_compound` | 582 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `fix_movedcompoundpoint` | 594 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `init_linepointmoving` | 632 | ported | moving a point works; a box corner resizes the attached lines' ends (`point.move`, `engine/links.rs`: D14; xfig does not link here) | |
| `e_movept.c` | `cancel_movept_box` | 720 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `fix_box` | 735 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `assign_newboxpoint` | 761 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_movept.c` | `cancel_movedlinepoint` | 793 | ported | moving a point works; a box corner resizes the attached lines' ends (`point.move`, `engine/links.rs`: D14; xfig does not link here) | |
| `e_movept.c` | `fix_movedlinepoint` | 808 | ported | moving a point works; a box corner resizes the attached lines' ends (`point.move`, `engine/links.rs`: D14; xfig does not link here) | |
| `e_movept.c` | `relocate_linepoint` | 835 | ported | tools: `ToolKind::MovePoint` (`point_event`, `commit_point`), command `point.move` |  |
| `e_placelib.c` | `put_selected` | 70 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `transform_lib_obj` | 119 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `sel_place_lib_obj_proc` | 152 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `sel_place_lib_obj` | 164 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `put_draw` | 180 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `change_draw_mode` | 199 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `place_lib_object_orig` | 216 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `place_lib_object` | 239 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `move_object` | 276 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `init_move_object` | 300 | missing | Place a library object (drawing mode "library") | D6 |
| `e_placelib.c` | `cancel_place_lib_obj` | 315 | missing | Place a library object (drawing mode "library") | D6 |
| `e_rotate.c` | `rotate_cw_selected` | 78 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_ccw_selected` | 93 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_selected` | 108 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `set_unset_center` | 125 | missing | rotation centre (anchor) chosen with the right button, and the number of copies | D10 |
| `e_rotate.c` | `init_rotate` | 148 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `init_copynrotate` | 160 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_search` | 176 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `init_rotateline` | 212 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `init_rotatetext` | 234 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `init_rotateellipse` | 253 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `init_rotatearc` | 274 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `init_rotatespline` | 295 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `init_rotatecompound` | 316 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_line` | 342 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_figure` | 361 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_spline` | 374 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_text` | 392 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_ellipse` | 412 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_arc` | 439 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `valid_rot_angle` | 483 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_compound` | 500 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_point` | 529 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_rotate.c` | `rotate_xy` | 553 | ported | commands `transform.rotate`, `transform.rotate_cw/ccw`; tools `RotateCw`, `RotateCcw` (also copy and rotate) |  |
| `e_scale.c` | `scale_selected` | 104 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_box_scale` | 124 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_center_scale` | 156 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `wrapup_scale` | 204 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_boxscale_ellipse` | 214 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `cancel_boxscale_ellipse` | 271 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `fix_boxscale_ellipse` | 284 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `boxrelocate_ellipsepoint` | 306 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_scale_ellipse` | 349 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `cancel_scale_ellipse` | 376 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `fix_scale_ellipse` | 385 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `relocate_ellipsepoint` | 400 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_scale_arc` | 428 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `cancel_scale_arc` | 448 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `fix_scale_arc` | 459 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `relocate_arcpoint` | 489 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_scale_spline` | 532 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `cancel_scale_spline` | 563 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `fix_scale_spline` | 572 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_boxscale_compound` | 595 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `cancel_boxscale_compound` | 682 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `fix_boxscale_compound` | 693 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_scale_compound` | 714 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `cancel_scale_compound` | 729 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `fix_scale_compound` | 740 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `prescale_compound` | 762 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `scale_compound` | 778 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `rescale_dimension_line` | 831 | missing | scaling dimension lines (ticks and text follow) | D10 |
| `e_scale.c` | `scale_line` | 1030 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `scale_spline` | 1058 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `scale_arc` | 1071 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `scale_ellipse` | 1113 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `scale_text` | 1136 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_boxscale_line` | 1158 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `cancel_boxscale_line` | 1256 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `fix_boxscale_line` | 1267 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `scale_radius` | 1301 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `init_scale_line` | 1320 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `cancel_scale_line` | 1354 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `fix_scale_line` | 1365 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `rescale_points` | 1397 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `scale_arrows` | 1424 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_scale.c` | `scale_arrow` | 1434 | ported | tools: `ToolKind::Scale` (`scale_event`, `commit_scale`: about the opposite corner and about the centre), `transform.scale`; scaling by an edge in one dimension is D10 | D10 |
| `e_tangent.c` | `tangent_selected` | 49 | ported | `cmd/tangent.rs` (`object.tangent_line`), `tools/tangent.rs` (`tool.tangent`); the length is `Indicator::tangent_length` | |
| `e_tangent.c` | `init_tangent_adding` | 68 | ported | `cmd/tangent.rs` (`object.tangent_line`), `tools/tangent.rs` (`tool.tangent`); the length is `Indicator::tangent_length` | |
| `e_tangent.c` | `init_normal_adding` | 76 | ported | `cmd/tangent.rs` (`object.tangent_line`), `tools/tangent.rs` (`tool.tangent`); the length is `Indicator::tangent_length` | |
| `e_tangent.c` | `tangent_or_normal` | 84 | ported | `cmd/tangent.rs` (`object.tangent_line`), `tools/tangent.rs` (`tool.tangent`); the length is `Indicator::tangent_length` | |
| `e_tangent.c` | `tangent_normal_line` | 113 | ported | `cmd/tangent.rs` (`object.tangent_line`), `tools/tangent.rs` (`tool.tangent`); the length is `Indicator::tangent_length` | |
| `e_update.c` | `update_selected` | 92 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `get_arrow_mode` | 111 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `get_arrow_type` | 124 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `init_update_settings` | 143 | ported | Update settings (middle button) honour the update flags (`Indicator::take_from`) |  |
| `e_update.c` | `up_from_arrow` | 315 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `init_update_object` | 334 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_ellipse` | 433 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_arc` | 449 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_line` | 480 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_text` | 506 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_spline` | 542 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `fix_fillstyle` | 565 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `up_arrow` | 580 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_compound` | 659 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_arcs` | 767 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_compounds` | 775 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_ellipses` | 783 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_lines` | 791 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_splines` | 799 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `update_texts` | 807 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `e_update.c` | `up_dashdot` | 815 | ported | tools: `ToolKind::Update` (`click_event`), commands `object.update`, `indicator.*` (take attributes from the object) |  |
| `f_load.c` | `load_file` | 65 | ported | `cmd/file.rs` `file.open` (also keeps the figure offset and sets units, paper, grid from the file) |  |
| `f_load.c` | `update_settings` | 150 | ported | `cmd/file.rs` `file.open` (also keeps the figure offset and sets units, paper, grid from the file) |  |
| `f_load.c` | `merge_file` | 237 | ported | `cmd/file.rs` `file.merge` / `merge_doc` (colour mapping, one compound, offset) |  |
| `f_load.c` | `update_recent_list` | 302 | ported | recent files in `prefs` (`recent_files`, File menu 1 to 9) |  |
| `f_load.c` | `read_fail_message` | 359 | ported | read errors reach the user as `EngineError`/`FormatError` messages |  |
| `f_neuclrtab.c` | `neu_init` | 85 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `neu_init2` | 96 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `neu_pixel` | 135 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `neu_colrs` | 148 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `neu_clrtab` | 165 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `neu_map_pixel` | 184 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `neu_map_colrs` | 190 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `neu_dith_colrs` | 199 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `initnet` | 307 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `inxbuild` | 324 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `inxsearch` | 365 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `contest` | 421 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `altersingle` | 458 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `alterneigh` | 476 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `learn` | 514 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `unbiasnet` | 573 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_neuclrtab.c` | `cpyclrtab` | 587 | not needed | NeuQuant colour quantisation to fit pictures into the X colormap; the renderer is true colour |  |
| `f_picobj.c` | `init_stream` | 101 | replaced | picture files are read through fig2dev `util/readpics` (streams, `.gz`/`.Z` decompression) in `engine::import::load_picture` (DD5) |  |
| `f_picobj.c` | `free_stream` | 112 | replaced | picture files are read through fig2dev `util/readpics` (streams, `.gz`/`.Z` decompression) in `engine::import::load_picture` (DD5) |  |
| `f_picobj.c` | `file_on_disk` | 147 | replaced | `engine::import::load_picture` / `readpics` (type detection, size, status, picture cache per object) |  |
| `f_picobj.c` | `get_picture_status` | 250 | replaced | `engine::import::load_picture` / `readpics` (type detection, size, status, picture cache per object) |  |
| `f_picobj.c` | `read_picobj` | 315 | replaced | `engine::import::load_picture` / `readpics` (type detection, size, status, picture cache per object) |  |
| `f_picobj.c` | `open_stream` | 447 | replaced | picture files are read through fig2dev `util/readpics` (streams, `.gz`/`.Z` decompression) in `engine::import::load_picture` (DD5) |  |
| `f_picobj.c` | `close_stream` | 504 | replaced | picture files are read through fig2dev `util/readpics` (streams, `.gz`/`.Z` decompression) in `engine::import::load_picture` (DD5) |  |
| `f_picobj.c` | `rewind_stream` | 519 | replaced | picture files are read through fig2dev `util/readpics` (streams, `.gz`/`.Z` decompression) in `engine::import::load_picture` (DD5) |  |
| `f_picobj.c` | `uncompressed_content` | 558 | replaced | picture files are read through fig2dev `util/readpics` (streams, `.gz`/`.Z` decompression) in `engine::import::load_picture` (DD5) |  |
| `f_picobj.c` | `image_size` | 615 | replaced | `engine::import::load_picture` / `readpics` (type detection, size, status, picture cache per object) |  |
| `f_picobj.c` | `internal_path` | 683 | partial | relative picture names are resolved against the figure directory (`engine::import`); rewriting the names when the figure is saved in another folder is not | S4-3 |
| `f_picobj.c` | `xf_relpath` | 780 | partial | relative picture names are resolved against the figure directory (`engine::import`); rewriting the names when the figure is saved in another folder is not | S4-3 |
| `f_picobj.c` | `relative_path` | 843 | partial | relative picture names are resolved against the figure directory (`engine::import`); rewriting the names when the figure is saved in another folder is not | S4-3 |
| `f_picobj.c` | `xf_external_path` | 874 | partial | relative picture names are resolved against the figure directory (`engine::import`); rewriting the names when the figure is saved in another folder is not | S4-3 |
| `f_picobj.c` | `external_path` | 959 | partial | relative picture names are resolved against the figure directory (`engine::import`); rewriting the names when the figure is saved in another folder is not | S4-3 |
| `f_read.c` | `read_figc` | 135 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `contains_picture` | 170 | not needed | asks whether a figure contains pictures (for relative-path handling at save) |  |
| `f_read.c` | `read_fig` | 215 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `readfp_fig` | 242 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_return` | 489 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_objects` | 498 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `parse_papersize` | 616 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_colordef` | 643 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_arcobject` | 672 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_compoundobject` | 761 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_ellipseobject` | 877 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_lineobject` | 923 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_splineobject` | 1151 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_textobject` | 1343 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `backslash_count` | 1555 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `attach_comments` | 1571 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `read_line` | 1606 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `save_comment` | 1624 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `skip_line` | 1661 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `fix_angle` | 1672 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `fix_depth` | 1681 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `shift_figure` | 1698 | ported | figure offset on load and merge (`cmd/file.rs` `offset`, `Object::translate`); resolution scaling is fig2dev reader |  |
| `f_read.c` | `scale_figure` | 1754 | ported | figure offset on load and merge (`cmd/file.rs` `offset`, `Object::translate`); resolution scaling is fig2dev reader |  |
| `f_read.c` | `check_color` | 1771 | ported | user colours are mapped by RGB on merge (`merge_doc`); the reader keeps the file's own |  |
| `f_read.c` | `swap_colors` | 1785 | ported | user colours are mapped by RGB on merge (`merge_doc`); the reader keeps the file's own |  |
| `f_read.c` | `merge_colors` | 1855 | ported | user colours are mapped by RGB on merge (`merge_doc`); the reader keeps the file's own |  |
| `f_read.c` | `renumber_comp` | 1951 | ported | user colours are mapped by RGB on merge (`merge_doc`); the reader keeps the file's own |  |
| `f_read.c` | `renumber` | 1987 | ported | user colours are mapped by RGB on merge (`merge_doc`); the reader keeps the file's own |  |
| `f_read.c` | `count_lines_correctly` | 2008 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_read.c` | `convert_arrow` | 2026 | ported | fig2dev `read.rs` (port of `read.c`) through `format/fig_read` and `bridge/to_model` (DD3) |  |
| `f_readeps.c` | `scan_mediabox` | 60 | replaced | EPS/PDF pictures through fig2dev `util/{pdfpic,pdfps,gs}.rs` (PDF pages by hayro, ghostscript optional for EPS; B15) |  |
| `f_readeps.c` | `correct_boundingbox` | 101 | replaced | EPS/PDF pictures through fig2dev `util/{pdfpic,pdfps,gs}.rs` (PDF pages by hayro, ghostscript optional for EPS; B15) |  |
| `f_readeps.c` | `read_pdf` | 121 | replaced | EPS/PDF pictures through fig2dev `util/{pdfpic,pdfps,gs}.rs` (PDF pages by hayro, ghostscript optional for EPS; B15) |  |
| `f_readeps.c` | `read_eps` | 178 | replaced | EPS/PDF pictures through fig2dev `util/{pdfpic,pdfps,gs}.rs` (PDF pages by hayro, ghostscript optional for EPS; B15) |  |
| `f_readeps.c` | `hex` | 327 | replaced | EPS/PDF pictures through fig2dev `util/{pdfpic,pdfps,gs}.rs` (PDF pages by hayro, ghostscript optional for EPS; B15) |  |
| `f_readeps.c` | `lower` | 336 | replaced | EPS/PDF pictures through fig2dev `util/{pdfpic,pdfps,gs}.rs` (PDF pages by hayro, ghostscript optional for EPS; B15) |  |
| `f_readgif.c` | `read_gif` | 89 | replaced | GIF through fig2dev `util/readpics` (transparent colour kept; DD5) |  |
| `f_readgif.c` | `ReadColorMap` | 310 | replaced | GIF through fig2dev `util/readpics` (transparent colour kept; DD5) |  |
| `f_readgif.c` | `DoGIFextension` | 328 | replaced | GIF through fig2dev `util/readpics` (transparent colour kept; DD5) |  |
| `f_readgif.c` | `GetDataBlock` | 376 | replaced | GIF through fig2dev `util/readpics` (transparent colour kept; DD5) |  |
| `f_readjpg.c` | `read_jpg` | 89 | replaced | JPEG through fig2dev `util/readpics` (passthrough in PS/PDF) and the `image` crate |  |
| `f_readjpg.c` | `read_JPEG_file` | 108 | replaced | JPEG through fig2dev `util/readpics` (passthrough in PS/PDF) and the `image` crate |  |
| `f_readjpg.c` | `error_exit` | 323 | replaced | JPEG through fig2dev `util/readpics` (passthrough in PS/PDF) and the `image` crate |  |
| `f_readjpg.c` | `error_output` | 341 | replaced | JPEG through fig2dev `util/readpics` (passthrough in PS/PDF) and the `image` crate |  |
| `f_readold.c` | `read_1_3_objects` | 61 | ported | fig2dev `read/read1_3.rs` (G1: Fig 1.3 to 2.1 open in the app) |  |
| `f_readold.c` | `read_1_3_arcobject` | 147 | ported | fig2dev `read/read1_3.rs` (G1: Fig 1.3 to 2.1 open in the app) |  |
| `f_readold.c` | `read_1_3_compoundobject` | 189 | ported | fig2dev `read/read1_3.rs` (G1: Fig 1.3 to 2.1 open in the app) |  |
| `f_readold.c` | `read_1_3_ellipseobject` | 291 | ported | fig2dev `read/read1_3.rs` (G1: Fig 1.3 to 2.1 open in the app) |  |
| `f_readold.c` | `read_1_3_lineobject` | 329 | ported | fig2dev `read/read1_3.rs` (G1: Fig 1.3 to 2.1 open in the app) |  |
| `f_readold.c` | `read_1_3_splineobject` | 396 | ported | fig2dev `read/read1_3.rs` (G1: Fig 1.3 to 2.1 open in the app) |  |
| `f_readold.c` | `read_1_3_textobject` | 474 | ported | fig2dev `read/read1_3.rs` (G1: Fig 1.3 to 2.1 open in the app) |  |
| `f_readpcx.c` | `read_pcx` | 78 | replaced | PCX through fig2dev `util/readpics` (DD5) |  |
| `f_readpcx.c` | `_read_pcx` | 115 | replaced | PCX through fig2dev `util/readpics` (DD5) |  |
| `f_readpcx.c` | `dispbyte` | 286 | replaced | PCX through fig2dev `util/readpics` (DD5) |  |
| `f_readpng.c` | `user_error_fn` | 39 | replaced | PNG through fig2dev `util/readpics` (gamma, bKGD, tRNS; DD5) |  |
| `f_readpng.c` | `user_warning_fn` | 46 | replaced | PNG through fig2dev `util/readpics` (gamma, bKGD, tRNS; DD5) |  |
| `f_readpng.c` | `read_png` | 53 | replaced | PNG through fig2dev `util/readpics` (gamma, bKGD, tRNS; DD5) |  |
| `f_readppm.c` | `skip_comments_whitespace` | 42 | replaced | PPM/PGM/PBM through the `image` crate (feature `pnm`) |  |
| `f_readppm.c` | `read_8bitppm` | 66 | replaced | PPM/PGM/PBM through the `image` crate (feature `pnm`) |  |
| `f_readppm.c` | `scale_to_255` | 105 | replaced | PPM/PGM/PBM through the `image` crate (feature `pnm`) |  |
| `f_readppm.c` | `read6bytes` | 127 | replaced | PPM/PGM/PBM through the `image` crate (feature `pnm`) |  |
| `f_readppm.c` | `read_16bitppm` | 154 | replaced | PPM/PGM/PBM through the `image` crate (feature `pnm`) |  |
| `f_readppm.c` | `read_asciippm` | 190 | replaced | PPM/PGM/PBM through the `image` crate (feature `pnm`) |  |
| `f_readppm.c` | `read_ascii_max_ppm` | 240 | replaced | PPM/PGM/PBM through the `image` crate (feature `pnm`) |  |
| `f_readppm.c` | `read_ppm` | 295 | replaced | PPM/PGM/PBM through the `image` crate (feature `pnm`) |  |
| `f_readtif.c` | `error_handler` | 39 | replaced | TIFF through the `image` crate (feature `tiff`) |  |
| `f_readtif.c` | `read_tif` | 51 | replaced | TIFF through the `image` crate (feature `tiff`) |  |
| `f_readxbm.c` | `read_xbm` | 56 | replaced | XBM through fig2dev `util/readpics` (DD5) |  |
| `f_readxbm.c` | `initHexTable` | 130 | replaced | XBM through fig2dev `util/readpics` (DD5) |  |
| `f_readxbm.c` | `NextInt` | 164 | replaced | XBM through fig2dev `util/readpics` (DD5) |  |
| `f_readxbm.c` | `ReadDataFromBitmapFile` | 213 | replaced | XBM through fig2dev `util/readpics` (DD5) |  |
| `f_readxpm.c` | `read_xpm` | 48 | replaced | XPM through fig2dev `util/readpics` (DD5) |  |
| `f_save.c` | `init_write_tmpfile` | 58 | replaced | `cmd/file.rs` `file.save` writes through `std::fs` (and `.bak` is W10-9); no temporary file descriptors | W10-9 |
| `f_save.c` | `end_write_tmpfile` | 69 | replaced | `cmd/file.rs` `file.save` writes through `std::fs` (and `.bak` is W10-9); no temporary file descriptors | W10-9 |
| `f_save.c` | `write_fd` | 77 | replaced | `cmd/file.rs` `file.save` writes through `std::fs` (and `.bak` is W10-9); no temporary file descriptors | W10-9 |
| `f_save.c` | `write_file` | 90 | replaced | `cmd/file.rs` `file.save` writes through `std::fs` (and `.bak` is W10-9); no temporary file descriptors | W10-9 |
| `f_save.c` | `write_objects` | 125 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_objects_close` | 183 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_fig_header` | 196 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_colordefs` | 275 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_arc` | 298 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_compound` | 352 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_ellipse` | 397 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_line` | 436 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_spline` | 520 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_text` | 572 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_arrows` | 601 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `write_comments` | 626 | ported | `format/fig_write` (Fig 3.2 text, xfig's type order, colour definitions, arrows, comments) |  |
| `f_save.c` | `emergency_save` | 646 | replaced | `engine::recovery` (autosave and recovery list, A6) instead of a crash handler writing `.fig.SAVE` |  |
| `f_util.c` | `emptyname` | 77 | ported | empty-name and empty-figure checks and their messages in `cmd/file.rs` |  |
| `f_util.c` | `emptyname_msg` | 87 | ported | empty-name and empty-figure checks and their messages in `cmd/file.rs` |  |
| `f_util.c` | `emptyfigure` | 99 | ported | empty-name and empty-figure checks and their messages in `cmd/file.rs` |  |
| `f_util.c` | `emptyfigure_msg` | 117 | ported | empty-name and empty-figure checks and their messages in `cmd/file.rs` |  |
| `f_util.c` | `change_directory` | 129 | replaced | native file dialogs keep the directory (`rfd`; `FileDialogs`) |  |
| `f_util.c` | `get_directory` | 143 | replaced | native file dialogs keep the directory (`rfd`; `FileDialogs`) |  |
| `f_util.c` | `ok_to_write` | 178 | ported | overwrite check of `file.save`/`file.rename` (`overwrite` parameter, unsaved/exists dialog) |  |
| `f_util.c` | `xf_basename` | 223 | not needed | path helper (`Path::file_name`) |  |
| `f_util.c` | `remap_imagecolors` | 244 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `alloc_imagecolors` | 381 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `count_colors` | 398 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `count_pixels` | 410 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `readjust_cmap` | 422 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `free_pixmaps` | 443 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `extract_cmap` | 468 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `add_all_pixels` | 488 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `remap_image_colormap` | 512 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `map_to_mono` | 555 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `beep` | 666 | not needed | terminal bell on errors (egui has none; messages carry the news) |  |
| `f_util.c` | `safe_strcpy` | 676 | not needed | path helper (`Path::file_name`) |  |
| `f_util.c` | `read_xfigrc` | 693 | replaced | `~/.xfigrc` and `init_settings` become `settings.toml` (`engine::prefs`, E1); import of an old `~/.xfigrc` is S3 | E1, S3 |
| `f_util.c` | `add_recent_file` | 732 | ported | recent files list (`prefs::recent_files`) |  |
| `f_util.c` | `update_recent_files` | 754 | ported | recent files list (`prefs::recent_files`) |  |
| `f_util.c` | `update_xfigrc` | 776 | replaced | `~/.xfigrc` and `init_settings` become `settings.toml` (`engine::prefs`, E1); import of an old `~/.xfigrc` is S3 | E1, S3 |
| `f_util.c` | `strain_out` | 789 | replaced | `~/.xfigrc` and `init_settings` become `settings.toml` (`engine::prefs`, E1); import of an old `~/.xfigrc` is S3 | E1, S3 |
| `f_util.c` | `finish_update_xfigrc` | 838 | replaced | `~/.xfigrc` and `init_settings` become `settings.toml` (`engine::prefs`, E1); import of an old `~/.xfigrc` is S3 | E1, S3 |
| `f_util.c` | `init_settings` | 856 | replaced | `~/.xfigrc` and `init_settings` become `settings.toml` (`engine::prefs`, E1); import of an old `~/.xfigrc` is S3 | E1, S3 |
| `f_util.c` | `update_fig_files` | 975 | ported | `rustfig -update file.fig...` (`apps/rustfig`): rewrite the files in the current Fig version, keeping `.bak` | S3 |
| `f_util.c` | `build_command` | 1048 | replaced | no external programs are spawned for exports (fig2dev is ported, B0); the image editor is W10-16 | W10-16 |
| `f_util.c` | `strerror` | 1079 | not needed | path helper (`Path::file_name`) |  |
| `f_util.c` | `map_to_palette` | 1090 | not needed | X colormap and image colour reduction (pixel allocation, palette mapping) |  |
| `f_util.c` | `dimline_components` | 1156 | missing | dimension line parts (line, ticks, text of a dimension-line compound) | D10 |
| `f_util.c` | `find_smallest_depth` | 1182 | ported | depth range of a compound (`model` object depth queries; used by the compound edit panel) |  |
| `f_util.c` | `find_largest_depth` | 1217 | ported | depth range of a compound (`model` object depth queries; used by the compound edit panel) |  |
| `f_util.c` | `get_grid_spec` | 1253 | replaced | export/print grid fields use egui text fields; grid spec parsing in `fig2dev` options (`-G`) | W10-4 |
| `f_util.c` | `file_timestamp` | 1284 | not needed | file time stamp for the picture cache and Autorefresh (RustFig reads pictures at import; Autorefresh is W10-3) | W10-3 |
| `f_wrpcx.c` | `_write_pcx` | 49 | replaced | PCX writer in fig2dev `scene/bitmaps.rs` (B6) |  |
| `f_wrpcx.c` | `create_pcx_head` | 85 | replaced | PCX writer in fig2dev `scene/bitmaps.rs` (B6) |  |
| `f_wrpcx.c` | `putword` | 112 | replaced | PCX writer in fig2dev `scene/bitmaps.rs` (B6) |  |
| `f_wrpcx.c` | `write_pcx_head` | 119 | replaced | PCX writer in fig2dev `scene/bitmaps.rs` (B6) |  |
| `f_wrpcx.c` | `pcx_enc_scan` | 151 | replaced | PCX writer in fig2dev `scene/bitmaps.rs` (B6) |  |
| `f_wrpng.c` | `write_png` | 41 | replaced | PNG writer in fig2dev `scene/raster.rs` (B6) |  |
| `main.c` | `main` | 624 | ported | command line: `engine::cmdline` (option table, Xt abbreviations, `-help`, `-update`, `-geometry`); resources are `engine::prefs::RESOURCES` (E1); `~/.xfigrc` import left | S3 |
| `main.c` | `setup_visual` | 1626 | not needed | X visual and colormap set-up |  |
| `main.c` | `make_cut_buf_name` | 1744 | not needed | X cut-buffer property name (the clipboard is in memory, E7 for the system clipboard) |  |
| `main.c` | `check_resource_ranges` | 1769 | replaced | resource values are range-checked when set (`engine::prefs`) |  |
| `main.c` | `set_icon_geom` | 1798 | not needed | X window-manager icon geometry |  |
| `main.c` | `set_max_image_colors` | 1818 | not needed | X visual and colormap set-up |  |
| `main.c` | `parse_canvas_colors` | 1848 | not needed | X visual and colormap set-up |  |
| `main.c` | `set_xpm_icon` | 1901 | replaced | window icon from `app_icon.rs` |  |
| `main.c` | `resize_canvas` | 1959 | not needed | canvas size follows the egui layout |  |
| `main.c` | `get_pointer_mapping` | 2012 | replaced | mouse button mapping: `MouseScheme` and the left+right = middle chord (`input.rs`, W11-1) |  |
| `main.c` | `set_autorefresh` | 2037 | not needed | PROPOSAL: Autorefresh mode (View menu in xfig) redraws the canvas at an interval to clean up X11 drawing leftovers; egui redraws every frame. Task W10-3 records the decision | W10-3 |
| `main.c` | `cancel_autorefresh` | 2075 | not needed | PROPOSAL: Autorefresh mode (View menu in xfig) redraws the canvas at an interval to clean up X11 drawing leftovers; egui redraws every frame. Task W10-3 records the decision | W10-3 |
| `main.c` | `toggle_refresh_mode` | 2092 | not needed | PROPOSAL: Autorefresh mode (View menu in xfig) redraws the canvas at an interval to clean up X11 drawing leftovers; egui redraws every frame. Task W10-3 records the decision | W10-3 |
| `main.c` | `check_refresh` | 2109 | not needed | PROPOSAL: Autorefresh mode (View menu in xfig) redraws the canvas at an interval to clean up X11 drawing leftovers; egui redraws every frame. Task W10-3 records the decision | W10-3 |
| `mode.c` | `reset_modifiedflag` | 268 | ported | `Engine` modified flag (`is_modified`, set by every mutating command) |  |
| `mode.c` | `set_modifiedflag` | 274 | ported | `Engine` modified flag (`is_modified`, set by every mutating command) |  |
| `mode.c` | `set_action_on` | 280 | ported | `Engine::tool_busy` (a tool has an action in progress) |  |
| `mode.c` | `reset_action_on` | 286 | ported | `Engine::tool_busy` (a tool has an action in progress) |  |
| `u_bound.c` | `arc_bound` | 55 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `compound_bound` | 168 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `active_compound_bound` | 173 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `ellipse_bound` | 320 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `line_bound` | 430 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `spline_bound` | 440 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `general_spline_bound` | 457 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `approx_spline_bound` | 505 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `text_bound` | 524 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `points_bound` | 533 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `overlapping` | 554 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_bound.c` | `arrow_bound` | 569 | ported | fig2dev `bound.rs` through `format/bridge/bounds.rs` (DD2); the model keeps `Object::geometry_bbox` and `approx_bbox` |  |
| `u_colors.c` | `xtoxftcolor` | 95 | not needed | X/Xft colour allocation and pixel lookup; the colour tables (`colors`) are `figbase` colour tables |  |
| `u_colors.c` | `write_xftcolor_true` | 107 | not needed | X/Xft colour allocation and pixel lookup; the colour tables (`colors`) are `figbase` colour tables |  |
| `u_colors.c` | `write_xftcolor_nontrue` | 128 | not needed | X/Xft colour allocation and pixel lookup; the colour tables (`colors`) are `figbase` colour tables |  |
| `u_colors.c` | `setcolor_fromXColor` | 148 | not needed | X/Xft colour allocation and pixel lookup; the colour tables (`colors`) are `figbase` colour tables |  |
| `u_colors.c` | `check_colors` | 165 | not needed | X/Xft colour allocation and pixel lookup; the colour tables (`colors`) are `figbase` colour tables |  |
| `u_colors.c` | `getpixel` | 285 | not needed | X/Xft colour allocation and pixel lookup; the colour tables (`colors`) are `figbase` colour tables |  |
| `u_colors.c` | `getred` | 291 | not needed | X/Xft colour allocation and pixel lookup; the colour tables (`colors`) are `figbase` colour tables |  |
| `u_colors.c` | `getgreen` | 297 | not needed | X/Xft colour allocation and pixel lookup; the colour tables (`colors`) are `figbase` colour tables |  |
| `u_colors.c` | `getblue` | 303 | not needed | X/Xft colour allocation and pixel lookup; the colour tables (`colors`) are `figbase` colour tables |  |
| `u_convert.c` | `get_local_charset` | 58 | replaced | Rust strings are UTF-8; Fig text bytes are kept raw (`Text::raw`) and the reader decodes them (fig2dev `-E` charsets) |  |
| `u_convert.c` | `init_conversion` | 111 | replaced | Rust strings are UTF-8; Fig text bytes are kept raw (`Text::raw`) and the reader decodes them (fig2dev `-E` charsets) |  |
| `u_convert.c` | `convert` | 142 | replaced | Rust strings are UTF-8; Fig text bytes are kept raw (`Text::raw`) and the reader decodes them (fig2dev `-E` charsets) |  |
| `u_convert.c` | `conv_utf8strdup` | 226 | replaced | Rust strings are UTF-8; Fig text bytes are kept raw (`Text::raw`) and the reader decodes them (fig2dev `-E` charsets) |  |
| `u_convert.c` | `conv_strutf8dup` | 248 | replaced | Rust strings are UTF-8; Fig text bytes are kept raw (`Text::raw`) and the reader decodes them (fig2dev `-E` charsets) |  |
| `u_create.c` | `create_arrow` | 54 | ported | default arrows of the indicator (`Indicator`, model `Arrow`) |  |
| `u_create.c` | `forward_arrow` | 64 | ported | default arrows of the indicator (`Indicator`, model `Arrow`) |  |
| `u_create.c` | `backward_arrow` | 88 | ported | default arrows of the indicator (`Indicator`, model `Arrow`) |  |
| `u_create.c` | `new_arrow` | 112 | ported | default arrows of the indicator (`Indicator`, model `Arrow`) |  |
| `u_create.c` | `copy_comments` | 149 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `new_link` | 163 | ported | smart-link records between shapes and line ends (`links::Link`; D14: every shape kind) |  |
| `u_create.c` | `create_point` | 181 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_sfactor` | 196 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `copy_points` | 209 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `copy_sfactors` | 234 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `reverse_points` | 261 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `reverse_sfactors` | 294 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_arc` | 325 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `copy_arc` | 354 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_ellipse` | 391 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `copy_ellipse` | 406 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_line` | 426 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_pic` | 446 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_picture_entry` | 463 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `copy_line` | 481 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_spline` | 557 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `copy_spline` | 572 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_text` | 625 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `new_string` | 644 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `copy_text` | 654 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_compound` | 681 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `copy_compound` | 710 | not needed | allocation and deep copies (Rust `Clone`, `Vec`, `Arc`) |  |
| `u_create.c` | `create_dimension_line` | 795 | missing | dimension line construction (line, ticks, arrows, text) | D10 |
| `u_create.c` | `create_tick` | 952 | missing | dimension line construction (line, ticks, arrows, text) | D10 |
| `u_create.c` | `create_dimline_ticks` | 987 | missing | dimension line construction (line, ticks, arrows, text) | D10 |
| `u_create.c` | `backward_dim_arrow` | 999 | missing | dimension line construction (line, ticks, arrows, text) | D10 |
| `u_create.c` | `forward_dim_arrow` | 1023 | missing | dimension line construction (line, ticks, arrows, text) | D10 |
| `u_drag.c` | `init_ellipsedragging` | 52 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `cancel_ellipse` | 72 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `array_place_ellipse` | 91 | missing | array copy: place a grid of copies with Number of Copies X/Y | D10 |
| `u_drag.c` | `place_ellipse` | 146 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `place_ellipse_x` | 155 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `init_arcdragging` | 185 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `cancel_drag_arc` | 200 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `array_place_arc` | 219 | missing | array copy: place a grid of copies with Number of Copies X/Y | D10 |
| `u_drag.c` | `place_arc` | 273 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `place_arc_x` | 282 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `init_linedragging` | 312 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `cancel_line` | 333 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `array_place_line` | 353 | missing | array copy: place a grid of copies with Number of Copies X/Y | D10 |
| `u_drag.c` | `place_line` | 405 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `place_line_x` | 414 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `init_textdragging` | 452 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `array_place_text` | 482 | missing | array copy: place a grid of copies with Number of Copies X/Y | D10 |
| `u_drag.c` | `cancel_text` | 536 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `place_text` | 557 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `place_text_x` | 566 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `init_splinedragging` | 596 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `cancel_spline` | 611 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `array_place_spline` | 630 | missing | array copy: place a grid of copies with Number of Copies X/Y | D10 |
| `u_drag.c` | `place_spline` | 684 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `place_spline_x` | 693 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `init_compounddragging` | 723 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `cancel_drag_compound` | 743 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `array_place_compound` | 763 | missing | array copy: place a grid of copies with Number of Copies X/Y | D10 |
| `u_drag.c` | `place_compound` | 817 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_drag.c` | `place_compound_x` | 827 | ported | Move and Copy dragging: `move_event`, `commit_move`, preview from `Engine::overlay` |  |
| `u_draw.c` | `init_point_array` | 225 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `add_point` | 231 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `draw_point_array` | 270 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `draw_arc` | 278 | replaced | the canvas draws the scene of `rustfig_render::build_scene` (fig2dev display list and rasteriser): arcs, ellipses, lines, arc boxes, splines, texts, compounds |  |
| `u_draw.c` | `draw_ellipse` | 366 | replaced | the canvas draws the scene of `rustfig_render::build_scene` (fig2dev display list and rasteriser): arcs, ellipses, lines, arc boxes, splines, texts, compounds |  |
| `u_draw.c` | `add_closepoint` | 404 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `angle_ellipse` | 520 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `newpoint` | 618 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `draw_line` | 641 | replaced | the canvas draws the scene of `rustfig_render::build_scene` (fig2dev display list and rasteriser): arcs, ellipses, lines, arc boxes, splines, texts, compounds |  |
| `u_draw.c` | `draw_arcbox` | 807 | replaced | the canvas draws the scene of `rustfig_render::build_scene` (fig2dev display list and rasteriser): arcs, ellipses, lines, arc boxes, splines, texts, compounds |  |
| `u_draw.c` | `subset` | 843 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `draw_pic_pixmap` | 849 | replaced | pictures are rasterised by the renderer and cached as textures per object |  |
| `u_draw.c` | `create_pic_pixmap` | 953 | replaced | pictures are rasterised by the renderer and cached as textures per object |  |
| `u_draw.c` | `clr_mask_bit` | 1309 | replaced | pictures are rasterised by the renderer and cached as textures per object |  |
| `u_draw.c` | `draw_text` | 1323 | replaced | the canvas draws the scene of `rustfig_render::build_scene` (fig2dev display list and rasteriser): arcs, ellipses, lines, arc boxes, splines, texts, compounds |  |
| `u_draw.c` | `greek_text` | 1410 | replaced | Symbol/Dingbats text through the `text` crate glyph outlines |  |
| `u_draw.c` | `draw_compoundelements` | 1460 | replaced | the canvas draws the scene of `rustfig_render::build_scene` (fig2dev display list and rasteriser): arcs, ellipses, lines, arc boxes, splines, texts, compounds |  |
| `u_draw.c` | `compute_arcarrow_angle` | 1511 | ported | arrowheads: `figbase` `geom/arrow.rs` from fig2dev `bound.c` (`calc_arrow`, `clip_arrows`; D9) |  |
| `u_draw.c` | `tempXErrorHandler` | 1545 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `clip_arrows` | 1571 | ported | arrowheads: `figbase` `geom/arrow.rs` from fig2dev `bound.c` (`calc_arrow`, `clip_arrows`; D9) |  |
| `u_draw.c` | `calc_arrow` | 1723 | ported | arrowheads: `figbase` `geom/arrow.rs` from fig2dev `bound.c` (`calc_arrow`, `clip_arrows`; D9) |  |
| `u_draw.c` | `draw_arrow` | 1948 | ported | arrowheads: `figbase` `geom/arrow.rs` from fig2dev `bound.c` (`calc_arrow`, `clip_arrows`; D9) |  |
| `u_draw.c` | `curve` | 2008 | replaced | the canvas draws the scene of `rustfig_render::build_scene` (fig2dev display list and rasteriser): arcs, ellipses, lines, arc boxes, splines, texts, compounds |  |
| `u_draw.c` | `redraw_images` | 2127 | replaced | pictures are rasterised by the renderer and cached as textures per object |  |
| `u_draw.c` | `too_many_points` | 2141 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `debug_depth` | 2146 | not needed | X point arrays, X error handler, debug aids, X-specific helpers |  |
| `u_draw.c` | `draw_spline` | 2166 | replaced | the canvas draws the scene of `rustfig_render::build_scene` (fig2dev display list and rasteriser): arcs, ellipses, lines, arc boxes, splines, texts, compounds |  |
| `u_draw.c` | `quick_draw_spline` | 2217 | replaced | the canvas draws the scene of `rustfig_render::build_scene` (fig2dev display list and rasteriser): arcs, ellipses, lines, arc boxes, splines, texts, compounds |  |
| `u_draw_spline.c` | `f_blend` | 49 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `g_blend` | 59 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `h_blend` | 66 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `negative_s1_influence` | 73 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `negative_s2_influence` | 80 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `positive_s1_influence` | 87 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `positive_s2_influence` | 99 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `point_adding` | 111 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `point_computing` | 122 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `step_computing` | 133 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `spline_segment_computing` | 233 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `compute_open_spline` | 296 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_draw_spline.c` | `compute_closed_spline` | 342 | replaced | X-spline evaluation written from the paper in `figbase` `geom/xspline.rs` and fig2dev `trans_spline.rs` (port of `trans_spline.c`); review of the step size is open (S0 note) |  |
| `u_elastic.c` | `elastic_box` | 68 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_fixedbox` | 80 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_movebox` | 86 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `moving_box` | 99 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `resizing_box` | 108 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `constrained_resizing_box` | 118 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `constrained_resizing_scale_box` | 127 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `scaling_compound` | 136 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_scale_curcompound` | 144 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_scalecompound` | 150 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_line` | 176 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `unconstrained_line` | 189 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `latex_line` | 199 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_elastic.c` | `constrainedangle_line` | 216 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_elastic.c` | `angle0_line` | 265 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_elastic.c` | `angle90_line` | 273 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_elastic.c` | `angle45_line` | 281 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_elastic.c` | `angle135_line` | 293 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_elastic.c` | `reshaping_line` | 305 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_linelink` | 322 | ported | smart links: every attached line follows while dragging (`links::preview`, `Engine::overlay`; D8, D14) | |
| `u_elastic.c` | `moving_line` | 335 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_movenewline` | 344 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_moveline` | 350 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_links` | 379 | ported | smart links: every attached line follows while dragging (`links::preview`, `Engine::overlay`; D8, D14) | |
| `u_elastic.c` | `scaling_line` | 421 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_scale_curline` | 433 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `scaling_spline` | 441 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_scale_curspline` | 449 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_scalepts` | 455 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_poly` | 483 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `resizing_poly` | 515 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_ebr` | 528 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `resizing_ebr` | 549 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `constrained_resizing_ebr` | 559 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_ebd` | 568 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `resizing_ebd` | 587 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `constrained_resizing_ebd` | 597 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_cbr` | 606 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `resizing_cbr` | 623 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_cbd` | 633 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `resizing_cbd` | 651 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `constrained_resizing_cbd` | 661 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_moveellipse` | 670 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `moving_ellipse` | 689 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_scaleellipse` | 698 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `scaling_ellipse` | 732 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_scale_curellipse` | 746 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `arc_point` | 754 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `reshaping_arc` | 765 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_arclink` | 783 | ported | smart links: every attached line follows while dragging (`links::preview`, `Engine::overlay`; D8, D14) | |
| `u_elastic.c` | `moving_arc` | 804 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_movenewarc` | 813 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_movearc` | 819 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `scaling_arc` | 834 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_scale_curarc` | 842 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_scalearc` | 848 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `moving_text` | 884 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_movetext` | 907 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `moving_spline` | 918 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `elastic_movenewspline` | 927 | replaced | rubber bands are drawn from `Engine::overlay()` for every tool: boxes, lines, polygons, ellipses, circles, arcs, moves, scales |  |
| `u_elastic.c` | `adjust_box_pos` | 935 | ported | positioning magnet and grid snap (`Engine::snap`, `view.set_snap`) |  |
| `u_elastic.c` | `adjust_pos` | 990 | ported | positioning magnet and grid snap (`Engine::snap`, `view.set_snap`) |  |
| `u_error.c` | `error_handler` | 45 | not needed | X and Xt error handlers (no X); panics are caught by `engine::guard` |  |
| `u_error.c` | `X_error_handler` | 70 | not needed | X and Xt error handlers (no X); panics are caught by `engine::guard` |  |
| `u_error.c` | `emergency_quit` | 93 | replaced | crash handling: `engine::guard` plus the recovery files of `engine::recovery` (A6) |  |
| `u_error.c` | `my_quit` | 120 | replaced | crash handling: `engine::guard` plus the recovery files of `engine::recovery` (A6) |  |
| `u_fonts.c` | `fontnum` | 183 | ported | font number tables and lookups in `figbase` text / `rustfig_text` (PostScript and LaTeX font numbers, 35 families, flags) |  |
| `u_fonts.c` | `psfontnum` | 196 | ported | font number tables and lookups in `figbase` text / `rustfig_text` (PostScript and LaTeX font numbers, 35 families, flags) |  |
| `u_fonts.c` | `latexfontnum` | 208 | ported | font number tables and lookups in `figbase` text / `rustfig_text` (PostScript and LaTeX font numbers, 35 families, flags) |  |
| `u_fonts.c` | `font_close` | 221 | replaced | fonts are egui/craft-fonts faces loaded by the renderer; no X font open/close/cache |  |
| `u_fonts.c` | `font_closeall` | 274 | replaced | fonts are egui/craft-fonts faces loaded by the renderer; no X font open/close/cache |  |
| `u_fonts.c` | `font_open` | 318 | replaced | fonts are egui/craft-fonts faces loaded by the renderer; no X font open/close/cache |  |
| `u_fonts.c` | `font_xfcopy` | 452 | replaced | fonts are egui/craft-fonts faces loaded by the renderer; no X font open/close/cache |  |
| `u_fonts.c` | `_textextents` | 470 | ported | text metrics (ascent, descent, width, baseline box, origin) in `rustfig_text` / `figbase` text (fig2dev `bound.c` text bounds) |  |
| `u_fonts.c` | `font_textoffset` | 535 | ported | text metrics (ascent, descent, width, baseline box, origin) in `rustfig_text` / `figbase` text (fig2dev `bound.c` text bounds) |  |
| `u_fonts.c` | `font_setbbox_offset` | 548 | ported | text metrics (ascent, descent, width, baseline box, origin) in `rustfig_text` / `figbase` text (fig2dev `bound.c` text bounds) |  |
| `u_fonts.c` | `font_baselinebbox` | 610 | ported | text metrics (ascent, descent, width, baseline box, origin) in `rustfig_text` / `figbase` text (fig2dev `bound.c` text bounds) |  |
| `u_fonts.c` | `font_textlen` | 677 | ported | text metrics (ascent, descent, width, baseline box, origin) in `rustfig_text` / `figbase` text (fig2dev `bound.c` text bounds) |  |
| `u_fonts.c` | `font_ascentdescent` | 694 | ported | text metrics (ascent, descent, width, baseline box, origin) in `rustfig_text` / `figbase` text (fig2dev `bound.c` text bounds) |  |
| `u_fonts.c` | `text_origin` | 719 | ported | text metrics (ascent, descent, width, baseline box, origin) in `rustfig_text` / `figbase` text (fig2dev `bound.c` text bounds) |  |
| `u_fonts.c` | `font_base` | 747 | ported | text metrics (ascent, descent, width, baseline box, origin) in `rustfig_text` / `figbase` text (fig2dev `bound.c` text bounds) |  |
| `u_fonts.c` | `font_reload` | 771 | replaced | fonts are egui/craft-fonts faces loaded by the renderer; no X font open/close/cache |  |
| `u_fonts.c` | `map_dingbats` | 796 | ported | Symbol and ZapfDingbats character tables (`figbase` text, fig2dev SVG tables) |  |
| `u_fonts.c` | `map_symbols` | 1013 | ported | Symbol and ZapfDingbats character tables (`figbase` text, fig2dev SVG tables) |  |
| `u_fonts.c` | `_charmap` | 1223 | replaced | fonts are egui/craft-fonts faces loaded by the renderer; no X font open/close/cache |  |
| `u_fonts.c` | `font_drawtext` | 1240 | replaced | fonts are egui/craft-fonts faces loaded by the renderer; no X font open/close/cache |  |
| `u_free.c` | `free_arc` | 38 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_compound` | 57 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_ellipse` | 81 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_line` | 96 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_text` | 109 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_spline` | 127 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_splinestorage` | 140 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_linestorage` | 154 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_picture_entry` | 177 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_points` | 215 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_sfactors` | 226 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_linkinfo` | 236 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_GCs` | 251 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_Fonts` | 280 | not needed | memory management (Rust ownership) |  |
| `u_free.c` | `free_objects` | 312 | not needed | memory management (Rust ownership) |  |
| `u_geom.c` | `rotate_vector` | 40 | ported | `rustfig_geom` `Affine::rotate`, hit tests (`geom::hit`: distance to segment, arc, ellipse, polyline, spline) |  |
| `u_geom.c` | `compute_normal` | 61 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `close_to_vector` | 91 | ported | `rustfig_geom` `Affine::rotate`, hit tests (`geom::hit`: distance to segment, arc, ellipse, polyline, spline) |  |
| `u_geom.c` | `compute_arcradius` | 159 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_arccenter` | 198 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `close_to_arc` | 253 | ported | `rustfig_geom` `Affine::rotate`, hit tests (`geom::hit`: distance to segment, arc, ellipse, polyline, spline) |  |
| `u_geom.c` | `compute_ellipse_distance` | 334 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `close_to_ellipse` | 359 | ported | `rustfig_geom` `Affine::rotate`, hit tests (`geom::hit`: distance to segment, arc, ellipse, polyline, spline) |  |
| `u_geom.c` | `close_to_polyline` | 443 | ported | `rustfig_geom` `Affine::rotate`, hit tests (`geom::hit`: distance to segment, arc, ellipse, polyline, spline) |  |
| `u_geom.c` | `close_to_float_vector` | 496 | ported | `rustfig_geom` `Affine::rotate`, hit tests (`geom::hit`: distance to segment, arc, ellipse, polyline, spline) |  |
| `u_geom.c` | `init_point_array` | 554 | not needed | point array helpers and `atan2` |  |
| `u_geom.c` | `add_point` | 559 | not needed | point array helpers and `atan2` |  |
| `u_geom.c` | `add_closepoint` | 586 | not needed | point array helpers and `atan2` |  |
| `u_geom.c` | `close_to_spline` | 591 | ported | `rustfig_geom` `Affine::rotate`, hit tests (`geom::hit`: distance to segment, arc, ellipse, polyline, spline) |  |
| `u_geom.c` | `compute_angle` | 625 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_direction` | 663 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_3p_angle` | 692 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_line_angle` | 722 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_arc_angle` | 754 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_poly_length` | 798 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_arc_length` | 831 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_poly_area` | 857 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_arc_area` | 882 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `compute_ellipse_area` | 919 | ported | `cmd/measure.rs` (`angle_3p`, `line_angle`, `arc_angle`, `poly_length`, `arc_length`, `poly_area`, `arc_area`) | |
| `u_geom.c` | `pgcd` | 933 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_geom.c` | `gcd` | 944 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_geom.c` | `lcm` | 958 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_geom.c` | `get_slope` | 1015 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_geom.c` | `latex_endpoint` | 1052 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `u_ghostscript.c` | `stdin_void` | 129 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_ghostscript.c` | `stderr_buf` | 138 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_ghostscript.c` | `stdout_mediabox` | 157 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_ghostscript.c` | `gslib` | 205 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `gsexe` | 281 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_ghostscript.c` | `gsexe_mediabox` | 378 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_ghostscript.c` | `gslib_mediabox` | 469 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `gs_mediabox` | 556 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_ghostscript.c` | `gsexe_bitmap` | 591 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_ghostscript.c` | `bitmap_stderr` | 771 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_ghostscript.c` | `bitmap_stdout` | 792 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_ghostscript.c` | `display_open` | 827 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `display_presize` | 836 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `display_memalloc` | 855 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `display_size` | 870 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `display_sync` | 891 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `display_page` | 900 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `display_preclose` | 912 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `display_memfree` | 921 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `display_close` | 931 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `callout_function` | 967 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `gslib_bitmap` | 993 | not needed | ghostscript display-library (`libgs`) callbacks; RustFig runs only the executable |  |
| `u_ghostscript.c` | `gs_bitmap` | 1204 | replaced | EPS/PDF size and preview: fig2dev `util/{pdfpic,pdfps,gs}.rs` (ghostscript run as an executable when installed, PDF by hayro) |  |
| `u_list.c` | `list_delete_arc` | 49 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_delete_ellipse` | 73 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_delete_line` | 97 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_delete_spline` | 121 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_delete_text` | 145 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_delete_compound` | 168 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `remove_depth` | 193 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `remove_compound_depth` | 257 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_add_arc` | 294 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_add_ellipse` | 311 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_add_line` | 328 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_add_spline` | 345 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_add_text` | 362 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `list_add_compound` | 379 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `add_depth` | 401 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `add_compound_depth` | 459 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `delete_line` | 492 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `delete_arc` | 502 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `delete_ellipse` | 512 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `delete_text` | 522 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `delete_spline` | 532 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `delete_compound` | 542 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `add_line` | 556 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `add_arc` | 566 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `add_ellipse` | 576 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `add_text` | 586 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `add_spline` | 596 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `add_compound` | 606 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `change_line` | 617 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `change_arc` | 629 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `change_ellipse` | 641 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `change_text` | 653 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `change_spline` | 665 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `change_compound` | 677 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `tail` | 690 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `append_objects` | 732 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `cut_objects` | 765 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `remove_arc_depths` | 812 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `remove_ellipse_depths` | 819 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `remove_line_depths` | 826 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `remove_spline_depths` | 833 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `remove_text_depths` | 840 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `append_point` | 847 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `insert_point` | 862 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `append_sfactor` | 877 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `first_spline_point` | 891 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `num_points` | 911 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `last_text` | 922 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `last_line` | 935 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `last_spline` | 948 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `last_arc` | 961 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `last_ellipse` | 974 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `last_compound` | 987 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `last_point` | 1000 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `last_sfactor` | 1013 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `search_line_point` | 1026 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `search_spline_point` | 1036 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `search_sfactor` | 1047 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `prev_arc` | 1060 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `prev_compound` | 1073 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `prev_ellipse` | 1086 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `prev_line` | 1099 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `prev_spline` | 1112 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `prev_text` | 1125 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `prev_point` | 1138 | replaced | object lists per type and depth: `model::Document` (one list of `Arc<Object>` with ids; depth counts in the depth panel, draw order by `xfig_order`) |  |
| `u_list.c` | `object_count` | 1151 | ported | object counts per depth (`depth.*`, depth panel) |  |
| `u_list.c` | `set_tags` | 1177 | ported | tagging objects for glue and region commands (selection and `Band` tags) |  |
| `u_list.c` | `get_links` | 1219 | ported | linked move: collects the line ends on a moved box's perimeter (`engine/links.rs`) |  |
| `u_list.c` | `point_on_perim` | 1262 | ported | linked move: whether a point is within `LINK_TOL` of a box's perimeter (`engine/links.rs`) |  |
| `u_list.c` | `get_interior_links` | 1279 | ported | linked move: collects the line ends inside a moved compound (`engine/links.rs`) |  |
| `u_list.c` | `point_on_inside` | 1320 | ported | linked move: whether a point is inside a compound's box (`engine/links.rs`) |  |
| `u_list.c` | `adjust_links` | 1328 | ported | linked move: moves the attached line ends, and slides the adjacent point in slide mode, through any transform (`engine/links.rs`; D14) |  |
| `u_markers.c` | `center_marker` | 43 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_csrhighlight` | 55 | missing | highlight of the object found by a Shift-click and of the cursor position when nothing is found | S4-1 |
| `u_markers.c` | `ellipse_in_mask` | 62 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `arc_in_mask` | 67 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `compound_in_mask` | 72 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `anytext_in_mask` | 77 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `validtext_in_mask` | 82 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `anyline_in_mask` | 88 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `validline_in_mask` | 93 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `anyspline_in_mask` | 101 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `validspline_in_mask` | 106 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `mask_toggle_ellipsemarker` | 116 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `mask_toggle_arcmarker` | 122 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `mask_toggle_compoundmarker` | 128 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `mask_toggle_textmarker` | 134 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `mask_toggle_linemarker` | 140 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `mask_toggle_splinemarker` | 146 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_markers` | 152 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_markers_in_compound` | 157 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `update_markers` | 210 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_ellipsemarker` | 268 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_ellipsehighlight` | 277 | missing | highlight of the object found by a Shift-click and of the cursor position when nothing is found | S4-1 |
| `u_markers.c` | `toggle_arcmarker` | 286 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_archighlight` | 296 | missing | highlight of the object found by a Shift-click and of the cursor position when nothing is found | S4-1 |
| `u_markers.c` | `toggle_textmarker` | 307 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_texthighlight` | 323 | missing | highlight of the object found by a Shift-click and of the cursor position when nothing is found | S4-1 |
| `u_markers.c` | `toggle_all_compoundmarkers` | 342 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_compoundmarker` | 349 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_compoundhighlight` | 360 | missing | highlight of the object found by a Shift-click and of the cursor position when nothing is found | S4-1 |
| `u_markers.c` | `toggle_linemarker` | 373 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_linehighlight` | 395 | missing | highlight of the object found by a Shift-click and of the cursor position when nothing is found | S4-1 |
| `u_markers.c` | `toggle_splinemarker` | 417 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_markers.c` | `toggle_splinehighlight` | 438 | missing | highlight of the object found by a Shift-click and of the cursor position when nothing is found | S4-1 |
| `u_markers.c` | `toggle_pointmarker` | 459 | ported | `engine/tools/markers.rs` (`mode_mask`, `object_markers`, masks per mode, W12) and the canvas |  |
| `u_pan.c` | `pan_left` | 34 | ported | `ui-egui/rulers.rs` `pan_step`, commands `view.pan`, `view.pan_origin` |  |
| `u_pan.c` | `pan_right` | 42 | ported | `ui-egui/rulers.rs` `pan_step`, commands `view.pan`, `view.pan_origin` |  |
| `u_pan.c` | `pan_up` | 52 | ported | `ui-egui/rulers.rs` `pan_step`, commands `view.pan`, `view.pan_origin` |  |
| `u_pan.c` | `pan_down` | 60 | ported | `ui-egui/rulers.rs` `pan_step`, commands `view.pan`, `view.pan_origin` |  |
| `u_pan.c` | `pan_origin` | 71 | ported | `ui-egui/rulers.rs` `pan_step`, commands `view.pan`, `view.pan_origin` |  |
| `u_print.c` | `cmdlinetoargarray` | 72 | missing | printing: planned as a RustFig print dialog and a PDF handed to the OS print dialog (no spawned `lpr`) | B5 |
| `u_print.c` | `start_argumentlist` | 125 | missing | printing: planned as a RustFig print dialog and a PDF handed to the OS print dialog (no spawned `lpr`) | B5 |
| `u_print.c` | `addargs_postscript` | 157 | replaced | export runs fig2dev's ported drivers in process (`engine::fig2dev::export`, `file.export`); options map in `driver_args` |  |
| `u_print.c` | `border_arg` | 195 | replaced | export runs fig2dev's ported drivers in process (`engine::fig2dev::export`, `file.export`); options map in `driver_args` |  |
| `u_print.c` | `spawn_export_to_fd` | 208 | replaced | export runs fig2dev's ported drivers in process (`engine::fig2dev::export`, `file.export`); options map in `driver_args` |  |
| `u_print.c` | `spawn_exportcommand` | 225 | replaced | export runs fig2dev's ported drivers in process (`engine::fig2dev::export`, `file.export`); options map in `driver_args` |  |
| `u_print.c` | `print_to_printer` | 248 | missing | printing: planned as a RustFig print dialog and a PDF handed to the OS print dialog (no spawned `lpr`) | B5 |
| `u_print.c` | `print_to_batchfile` | 308 | missing | printing: planned as a RustFig print dialog and a PDF handed to the OS print dialog (no spawned `lpr`) | B5 |
| `u_print.c` | `strsub` | 336 | missing | printing: planned as a RustFig print dialog and a PDF handed to the OS print dialog (no spawned `lpr`) | B5 |
| `u_print.c` | `pass_environment` | 357 | missing | printing: planned as a RustFig print dialog and a PDF handed to the OS print dialog (no spawned `lpr`) | B5 |
| `u_print.c` | `print_export` | 383 | replaced | export runs fig2dev's ported drivers in process (`engine::fig2dev::export`, `file.export`); options map in `driver_args` |  |
| `u_print.c` | `print_spawn_printcmd` | 769 | missing | printing: planned as a RustFig print dialog and a PDF handed to the OS print dialog (no spawned `lpr`) | B5 |
| `u_print.c` | `make_rgb_string` | 825 | ported | background colour and active layers as export options (`driver_args`, `active_layers_only`) |  |
| `u_print.c` | `build_layer_list` | 840 | ported | background colour and active layers as export options (`driver_args`, `active_layers_only`) |  |
| `u_print.c` | `append_group` | 915 | ported | background colour and active layers as export options (`driver_args`, `active_layers_only`) |  |
| `u_quartic.c` | `quadratic` | 29 | replaced | intersections and tangents on flattened outlines (`engine/snap.rs`), no quartic solver |  |
| `u_quartic.c` | `cubic` | 99 | replaced | intersections and tangents on flattened outlines (`engine/snap.rs`), no quartic solver |  |
| `u_quartic.c` | `quartic` | 170 | replaced | intersections and tangents on flattened outlines (`engine/snap.rs`), no quartic solver |  |
| `u_redraw.c` | `clearallcounts` | 83 | ported | object counts per depth for the depth panel |  |
| `u_redraw.c` | `clearcounts` | 102 | ported | object counts per depth for the depth panel |  |
| `u_redraw.c` | `redisplay_objects` | 115 | ported | draw order within a depth and gray inactive layers (`rustfig_render::build_scene`, S1) |  |
| `u_redraw.c` | `redisplay_arcobject` | 203 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_ellipseobject` | 229 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_lineobject` | 256 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_splineobject` | 283 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_textobject` | 308 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_compoundobject` | 332 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_canvas` | 350 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_curobj` | 361 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_region` | 451 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `update_pageborder` | 478 | ported | page border (`view.toggle_page_border`, canvas) |  |
| `u_redraw.c` | `redisplay_pageborder` | 490 | ported | page border (`view.toggle_page_border`, canvas) |  |
| `u_redraw.c` | `redraw_pageborder` | 509 | ported | page border (`view.toggle_page_border`, canvas) |  |
| `u_redraw.c` | `draw_pb` | 576 | ported | page border (`view.toggle_page_border`, canvas) |  |
| `u_redraw.c` | `redisplay_zoomed_region` | 584 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_ellipse` | 589 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_ellipses` | 597 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_arc` | 607 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_arcs` | 636 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_spline` | 646 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_splines` | 654 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_line` | 664 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_lines` | 672 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_compound` | 682 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_compounds` | 688 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_text` | 696 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_texts` | 704 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_redraw.c` | `redisplay_regions` | 716 | replaced | redraw of single objects and regions: the canvas redraws the cached scene each frame (A7) |  |
| `u_scale.c` | `read_scale_arrow` | 26 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_ellipse` | 35 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_arc` | 47 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_line` | 62 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_text` | 79 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_spline` | 87 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_compound` | 100 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_arcs` | 115 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_compounds` | 123 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_ellipses` | 131 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_lines` | 139 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_splines` | 147 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_scale.c` | `read_scale_texts` | 155 | not needed | scaling of objects read from files with another resolution is done by fig2dev's reader |  |
| `u_search.c` | `next_arc_found` | 74 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `vector_rotate` | 142 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `next_ellipse_found` | 154 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `next_line_found` | 220 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `next_spline_found` | 273 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `next_text_found` | 316 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `next_compound_found` | 348 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `show_objecthighlight` | 383 | missing | Shift-click: highlight the object found and, with Shift held, the next overlapping one, before acting | S4-1 |
| `u_search.c` | `erase_objecthighlight` | 391 | missing | Shift-click: highlight the object found and, with Shift held, the next overlapping one, before acting | S4-1 |
| `u_search.c` | `toggle_objecthighlight` | 403 | missing | Shift-click: highlight the object found and, with Shift held, the next overlapping one, before acting | S4-1 |
| `u_search.c` | `init_search` | 430 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `do_object_search` | 463 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `object_search_left` | 556 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `object_search_middle` | 565 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `object_search_right` | 574 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `next_arc_point_found` | 583 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `next_ellipse_point_found` | 613 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `next_line_point_found` | 643 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `next_spline_point_found` | 675 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `next_compound_point_found` | 703 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `init_searchproc_left` | 749 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `init_searchproc_middle` | 755 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `init_searchproc_right` | 761 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `do_point_search` | 767 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `point_search_left` | 854 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `point_search_middle` | 863 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `point_search_right` | 872 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `text_search` | 881 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `in_text_bound` | 910 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `compound_search` | 944 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_search.c` | `compound_point_search` | 969 | ported | point search: `find_vertex`, `find_segment` (tools) for Move point, Add point, Delete point |  |
| `u_search.c` | `get_spline_point` | 1013 | ported | `Engine::pick` / `hit_test` with `geom::hit` per object kind; nearest object under the cursor in the mode's mask |  |
| `u_smartsearch.c` | `init_smart_searchproc_left` | 85 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `init_smart_searchproc_middle` | 91 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `init_smart_searchproc_right` | 97 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `smart_object_search_left` | 104 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `smart_object_search_middle` | 112 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `smart_object_search_right` | 119 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `set_smart_points` | 128 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `init_smart_search` | 137 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `do_smart_object_search` | 170 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `smart_next_arc_found` | 266 | ported | the curve point and tangent for Tangent mode (`cmd/tangent.rs`, D4); the smart-links search is `engine/links.rs` | |
| `u_smartsearch.c` | `smart_next_ellipse_found` | 293 | ported | the curve point and tangent for Tangent mode (`cmd/tangent.rs`, D4); the smart-links search is `engine/links.rs` | |
| `u_smartsearch.c` | `smart_next_line_found` | 328 | ported | the curve point and tangent for Tangent mode (`cmd/tangent.rs`, D4); the smart-links search is `engine/links.rs` | |
| `u_smartsearch.c` | `smart_next_spline_found` | 358 | ported | the curve point and tangent for Tangent mode (`cmd/tangent.rs`, D4); the smart-links search is `engine/links.rs` | |
| `u_smartsearch.c` | `smart_next_text_found` | 388 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `smart_next_compound_found` | 417 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `smart_show_objecthighlight` | 455 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `smart_erase_objecthighlight` | 463 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `smart_toggle_objecthighlight` | 475 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_smartsearch.c` | `smart_null_proc` | 502 | missing | object search for smart links (find the box and line end to move together) | D8 |
| `u_spawn.c` | `add_info` | 58 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `retrieve_info` | 85 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `full_command` | 129 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `spawn_process` | 156 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `open_process` | 220 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `closefderr` | 255 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `wait_pid` | 262 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `poll_fderr` | 292 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `spawn_exists` | 348 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `spawn_usefd` | 457 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `spawn_popen_fd` | 479 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `spawn_popen` | 533 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_spawn.c` | `spawn_pclose` | 544 | replaced | no external programs for exports (ported fig2dev); the image editor and help viewer use `std::process::Command` / the OS opener (W10-16, W10-2) | W10-16, W10-2 |
| `u_translate.c` | `translate_ellipse` | 32 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_arc` | 42 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_line` | 54 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_text` | 64 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_spline` | 75 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_compound` | 86 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_arcs` | 102 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_compounds` | 111 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_ellipses` | 120 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_lines` | 129 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_splines` | 138 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_translate.c` | `translate_texts` | 147 | ported | model `Object::translate` (every kind, compounds recursively) |  |
| `u_undo.c` | `undo` | 115 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_join_split` | 185 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_addpoint` | 226 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_deletepoint` | 236 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_break` | 255 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_glue` | 266 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_convert` | 280 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_add_arrowhead` | 296 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_delete_arrowhead` | 316 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_change` | 351 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_add` | 502 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_delete` | 540 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_move` | 594 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_load` | 644 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_scale` | 671 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `undo_open_close` | 687 | replaced | multi-level snapshot undo/redo with shared objects (`engine/history.rs`, A1); each command is one step |  |
| `u_undo.c` | `swap_newp_lastp` | 724 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `clean_up` | 744 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_latestarc` | 863 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_latestobjects` | 868 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_latestcompound` | 873 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_latestellipse` | 878 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_latestline` | 883 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_latestspline` | 888 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_latesttext` | 893 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_last_prevpoint` | 898 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_last_selectedpoint` | 903 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_last_selectedsfactor` | 908 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_last_nextpoint` | 913 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_last_arcpointnum` | 918 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_lastposition` | 923 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_newposition` | 929 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_action` | 935 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_action_object` | 940 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_lastlinkinfo` | 946 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_last_tension` | 952 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `u_undo.c` | `set_last_arrows` | 958 | not needed | undo bookkeeping of the last action (objects, points, positions, tension, arrows) and freeing |  |
| `w_browse.c` | `browse_panel_dismiss` | 99 | replaced | picture file browsing uses the native file dialog (`rfd`); the picture panel Browse button is W10-16 | W10-16 |
| `w_browse.c` | `got_browse` | 117 | replaced | picture file browsing uses the native file dialog (`rfd`); the picture panel Browse button is W10-16 | W10-16 |
| `w_browse.c` | `browse_panel_close` | 176 | replaced | picture file browsing uses the native file dialog (`rfd`); the picture panel Browse button is W10-16 | W10-16 |
| `w_browse.c` | `popup_browse_panel` | 183 | replaced | picture file browsing uses the native file dialog (`rfd`); the picture panel Browse button is W10-16 | W10-16 |
| `w_browse.c` | `create_browse_panel` | 234 | replaced | picture file browsing uses the native file dialog (`rfd`); the picture panel Browse button is W10-16 | W10-16 |
| `w_canvas.c` | `null_proc` | 111 | not needed | Xt no-op action |  |
| `w_canvas.c` | `canvas_exposed` | 119 | replaced | canvas events (`canvas.rs`, `input.rs`): expose, init, actions, input dispatch and keystrokes feed `Engine::tool_event` |  |
| `w_canvas.c` | `init_canvas` | 186 | replaced | canvas events (`canvas.rs`, `input.rs`): expose, init, actions, input dispatch and keystrokes feed `Engine::tool_event` |  |
| `w_canvas.c` | `add_canvas_actions` | 212 | replaced | canvas events (`canvas.rs`, `input.rs`): expose, init, actions, input dispatch and keystrokes feed `Engine::tool_event` |  |
| `w_canvas.c` | `setup_canvas` | 220 | replaced | canvas events (`canvas.rs`, `input.rs`): expose, init, actions, input dispatch and keystrokes feed `Engine::tool_event` |  |
| `w_canvas.c` | `get_input` | 231 | replaced | canvas events (`canvas.rs`, `input.rs`): expose, init, actions, input dispatch and keystrokes feed `Engine::tool_event` |  |
| `w_canvas.c` | `process_keystroke` | 270 | replaced | canvas events (`canvas.rs`, `input.rs`): expose, init, actions, input dispatch and keystrokes feed `Engine::tool_event` |  |
| `w_canvas.c` | `canvas_selected` | 327 | partial | button and motion handling ported (snap, Alt+right = middle, wheel pans, arrows pan); missing: Ctrl+click zoom (E3), Ctrl+Shift+click changes a spline point's shape factor in edit modes (S4-6), the full-window crosshair (resource `crosshair`, E9), coordinates typed on the keyboard panel (D10) | S4-6, E3, E9, D10 |
| `w_canvas.c` | `clear_canvas` | 662 | not needed | clearing the X window |  |
| `w_canvas.c` | `clear_region` | 678 | not needed | clearing the X window |  |
| `w_canvas.c` | `get_canvas_clipboard` | 685 | partial | text paste: the system clipboard pastes into the text tool through egui text events; middle-button paste of the primary selection is not | E10 |
| `w_canvas.c` | `paste_primary_selection` | 720 | partial | text paste: the system clipboard pastes into the text tool through egui text events; middle-button paste of the primary selection is not | E10 |
| `w_canvas.c` | `canvas_paste` | 728 | partial | text paste: the system clipboard pastes into the text tool through egui text events; middle-button paste of the primary selection is not | E10 |
| `w_canvas.c` | `getComposeKey` | 751 | ported | compose key: `menus::Compose`, `rustfig_text::compose` (CompKeyDB; W8) |  |
| `w_canvas.c` | `readComposeKey` | 769 | ported | compose key: `menus::Compose`, `rustfig_text::compose` (CompKeyDB; W8) |  |
| `w_canvas.c` | `setCompLED` | 878 | not needed | compose-key LED in the X window (RustFig has none) |  |
| `w_canvas.c` | `toggle_show_lengths` | 890 | missing | View > Show lengths (lengths and angles while drawing) | D10 |
| `w_canvas.c` | `toggle_show_vertexnums` | 900 | missing | View > Show vertex numbers | W10-3 |
| `w_canvas.c` | `toggle_show_borders` | 919 | ported | View > Show page borders (`view.toggle_page_border`) |  |
| `w_canvas.c` | `toggle_show_balloons` | 938 | ported | View > Show info balloons (`app.toggle_balloons`) |  |
| `w_canvas.c` | `popdown_mode_panel` | 958 | ported | Alt+wheel pop-up of the mode panel (`panels::mode_panel`) |  |
| `w_canvas.c` | `mode_panel_button_selected` | 965 | ported | Alt+wheel pop-up of the mode panel (`panels::mode_panel`) |  |
| `w_canvas.c` | `create_mode_panel` | 975 | ported | Alt+wheel pop-up of the mode panel (`panels::mode_panel`) |  |
| `w_canvas.c` | `popup_mode_panel` | 1034 | ported | Alt+wheel pop-up of the mode panel (`panels::mode_panel`) |  |
| `w_canvas.c` | `round_square` | 1064 | ported | grid and positioning-magnet rounding of points (`Engine::snap`, `view.set_snap`) |  |
| `w_canvas.c` | `point_spacing` | 1089 | ported | grid and positioning-magnet rounding of points (`Engine::snap`, `view.set_snap`) |  |
| `w_canvas.c` | `round_coords` | 1102 | ported | grid and positioning-magnet rounding of points (`Engine::snap`, `view.set_snap`) |  |
| `w_canvas.c` | `floor_coords` | 1158 | ported | grid and positioning-magnet rounding of points (`Engine::snap`, `view.set_snap`) |  |
| `w_canvas.c` | `ceil_coords` | 1189 | ported | grid and positioning-magnet rounding of points (`Engine::snap`, `view.set_snap`) |  |
| `w_capture.c` | `captureImage` | 71 | missing | Capture: grab a screen area into a picture object | D11 |
| `w_capture.c` | `rshift` | 149 | missing | Capture: grab a screen area into a picture object | D11 |
| `w_capture.c` | `getImageData` | 162 | missing | Capture: grab a screen area into a picture object | D11 |
| `w_capture.c` | `selectedRootArea` | 410 | missing | Capture: grab a screen area into a picture object | D11 |
| `w_capture.c` | `drawRect` | 527 | missing | Capture: grab a screen area into a picture object | D11 |
| `w_capture.c` | `getCurrentColors` | 584 | missing | Capture: grab a screen area into a picture object | D11 |
| `w_capture.c` | `canHandleCapture` | 660 | missing | Capture: grab a screen area into a picture object | D11 |
| `w_cmdpanel.c` | `num_main_menus` | 306 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `init_main_menus` | 313 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `add_cmd_actions` | 355 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `create_main_menu` | 361 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `rebuild_file_menu` | 389 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `create_menu_item` | 431 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `setup_main_menus` | 472 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `enter_cmd_but` | 491 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `quit` | 505 | ported | Quit with the unsaved-changes dialog (`app.quit`, `dialogs::unsaved_dialog`) |  |
| `w_cmdpanel.c` | `goodbye` | 532 | ported | Quit with the unsaved-changes dialog (`app.quit`, `dialogs::unsaved_dialog`) |  |
| `w_cmdpanel.c` | `paste` | 578 | partial | Paste: `edit.paste` pastes at a fixed offset; xfig attaches the objects to the mouse (left places, middle at the original position) | S4-2 |
| `w_cmdpanel.c` | `cancel_paste` | 676 | missing | placing pasted objects with the mouse | S4-2 |
| `w_cmdpanel.c` | `paste_draw` | 695 | missing | placing pasted objects with the mouse | S4-2 |
| `w_cmdpanel.c` | `move_paste_object` | 704 | missing | placing pasted objects with the mouse | S4-2 |
| `w_cmdpanel.c` | `init_move_paste_object` | 727 | missing | placing pasted objects with the mouse | S4-2 |
| `w_cmdpanel.c` | `place_object` | 741 | missing | placing pasted objects with the mouse | S4-2 |
| `w_cmdpanel.c` | `place_object_orig_posn` | 756 | missing | placing pasted objects with the mouse | S4-2 |
| `w_cmdpanel.c` | `new` | 775 | ported | `file.new`, `edit.delete_all`, `doc.toggle_orientation` |  |
| `w_cmdpanel.c` | `delete_all_cmd` | 804 | ported | `file.new`, `edit.delete_all`, `doc.toggle_orientation` |  |
| `w_cmdpanel.c` | `change_orient` | 827 | ported | `file.new`, `edit.delete_all`, `doc.toggle_orientation` |  |
| `w_cmdpanel.c` | `show_global_settings` | 924 | replaced | Global settings dialog `dialogs::global_dialog` and `resources_dialog` (every resource, E1); rows with effect are W10-23 | W10-23 |
| `w_cmdpanel.c` | `popup_global_panel` | 945 | replaced | Global settings dialog `dialogs::global_dialog` and `resources_dialog` (every resource, E1); rows with effect are W10-23 | W10-23 |
| `w_cmdpanel.c` | `create_global_panel` | 968 | replaced | Global settings dialog `dialogs::global_dialog` and `resources_dialog` (every resource, E1); rows with effect are W10-23 | W10-23 |
| `w_cmdpanel.c` | `CreateLabelledAscii` | 1136 | replaced | Global settings dialog `dialogs::global_dialog` and `resources_dialog` (every resource, E1); rows with effect are W10-23 | W10-23 |
| `w_cmdpanel.c` | `global_panel_done` | 1174 | replaced | Global settings dialog `dialogs::global_dialog` and `resources_dialog` (every resource, E1); rows with effect are W10-23 | W10-23 |
| `w_cmdpanel.c` | `global_panel_cancel` | 1290 | replaced | Global settings dialog `dialogs::global_dialog` and `resources_dialog` (every resource, E1); rows with effect are W10-23 | W10-23 |
| `w_cmdpanel.c` | `update_cur_filename` | 1304 | ported | window title and file name box (`update_cur_filename`) |  |
| `w_cmdpanel.c` | `popup_menu` | 1321 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `place_menu` | 1334 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `locate_menu` | 1357 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `load_recent_file` | 1376 | ported | File menu recent files 1 to 9 (`prefs::recent_files`) |  |
| `w_cmdpanel.c` | `acc_load_recent_file` | 1408 | ported | File menu recent files 1 to 9 (`prefs::recent_files`) |  |
| `w_cmdpanel.c` | `charmap_font_dimensions` | 1431 | missing | Character map panel inserting into the text being typed | E5 |
| `w_cmdpanel.c` | `refresh_character_panel` | 1458 | missing | Character map panel inserting into the text being typed | E5 |
| `w_cmdpanel.c` | `character_panel_close` | 1545 | missing | Character map panel inserting into the text being typed | E5 |
| `w_cmdpanel.c` | `popup_character_map` | 1564 | missing | Character map panel inserting into the text being typed | E5 |
| `w_cmdpanel.c` | `paste_character` | 1712 | missing | Character map panel inserting into the text being typed | E5 |
| `w_cmdpanel.c` | `redraw_character` | 1731 | missing | Character map panel inserting into the text being typed | E5 |
| `w_cmdpanel.c` | `redraw_all` | 1749 | replaced | redraw is automatic every frame (A7) |  |
| `w_cmdpanel.c` | `refresh_view_menu_item` | 1793 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_cmdpanel.c` | `refresh_view_menu` | 1828 | replaced | menu bar and menu items from `menus::XFIG_MENUS` and `LAYOUT` (egui menus) |  |
| `w_color.c` | `alloc_or_store_color` | 264 | not needed | X colormap cell allocation and private colormaps |  |
| `w_color.c` | `alloc_or_store_colors` | 306 | not needed | X colormap cell allocation and private colormaps |  |
| `w_color.c` | `create_color_panel` | 314 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `pen_fill_activate` | 872 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `xftcolor_fromcolor` | 884 | not needed | X colormap cell allocation and private colormaps |  |
| `w_color.c` | `restore_mixed_colors` | 893 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `set_mixed_color` | 943 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `set_user_color` | 957 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `pick_contrast` | 973 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `set_mixed_name` | 989 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `cancel_color_popup` | 1014 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `add_color` | 1026 | ported | `color.add`, `prefs.add_color`, `color.remove_unused`: user colours created, listed, counted and pruned when unused |  |
| `w_color.c` | `del_color` | 1047 | partial | delete a user colour exists only as remove-unused and `prefs.remove_color`; the editor's delete/undelete is E4 | E4 |
| `w_color.c` | `undel_color` | 1089 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `count_user_colors` | 1112 | ported | `color.add`, `prefs.add_color`, `color.remove_unused`: user colours created, listed, counted and pruned when unused |  |
| `w_color.c` | `c_user_colors` | 1139 | ported | `color.add`, `prefs.add_color`, `color.remove_unused`: user colours created, listed, counted and pruned when unused |  |
| `w_color.c` | `count_one` | 1174 | ported | `color.add`, `prefs.add_color`, `color.remove_unused`: user colours created, listed, counted and pruned when unused |  |
| `w_color.c` | `del_unused_user_colors` | 1183 | ported | `color.add`, `prefs.add_color`, `color.remove_unused`: user colours created, listed, counted and pruned when unused |  |
| `w_color.c` | `lookup_color` | 1231 | ported | colour names: `figbase` `x_color` (`lookup_X_color`) |  |
| `w_color.c` | `add_color_cell` | 1264 | not needed | X colormap cell allocation and private colormaps |  |
| `w_color.c` | `create_cell` | 1355 | not needed | X colormap cell allocation and private colormaps |  |
| `w_color.c` | `alloc_color_cells` | 1387 | not needed | X colormap cell allocation and private colormaps |  |
| `w_color.c` | `switch_colormap` | 1424 | not needed | X colormap cell allocation and private colormaps |  |
| `w_color.c` | `del_color_cell` | 1449 | not needed | X colormap cell allocation and private colormaps |  |
| `w_color.c` | `color_used` | 1474 | ported | `color.add`, `prefs.add_color`, `color.remove_unused`: user colours created, listed, counted and pruned when unused |  |
| `w_color.c` | `switch_edit` | 1506 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `set_slider_sensitivity` | 1539 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `set_color_ok` | 1550 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `_set_std_color` | 1569 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `set_std_color` | 1577 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `_pick_memory` | 1631 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `pick_memory` | 1646 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `lock_toggle` | 1697 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `draw_boxed` | 1739 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `erase_boxed` | 1751 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `color_borders` | 1765 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `WhichButton` | 1776 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `update_from_triple` | 1789 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `update_scrl_triple` | 1834 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `update_triple` | 1842 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `move_scroll` | 1868 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `StoreMix_and_Mem` | 1940 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `Scrolled` | 1965 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `Update_HSV` | 2017 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `ThumbHSV` | 2037 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `Thumbed` | 2060 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `move_lock` | 2106 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `next_pencolor` | 2119 | ported | stepping the pen/fill colour with the wheel on the indicator button (`attributes`) |  |
| `w_color.c` | `prev_pencolor` | 2131 | ported | stepping the pen/fill colour with the wheel on the indicator button (`attributes`) |  |
| `w_color.c` | `show_pencolor` | 2144 | ported | stepping the pen/fill colour with the wheel on the indicator button (`attributes`) |  |
| `w_color.c` | `next_fillcolor` | 2193 | ported | stepping the pen/fill colour with the wheel on the indicator button (`attributes`) |  |
| `w_color.c` | `prev_fillcolor` | 2205 | ported | stepping the pen/fill colour with the wheel on the indicator button (`attributes`) |  |
| `w_color.c` | `show_fillcolor` | 2218 | ported | stepping the pen/fill colour with the wheel on the indicator button (`attributes`) |  |
| `w_color.c` | `set_cmap` | 2268 | not needed | X colormap cell allocation and private colormaps |  |
| `w_color.c` | `HSVToRGB` | 2319 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `RGBToHSV` | 2363 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `PctToRGB` | 2406 | missing | colour editor: pen/fill panel, sliders (RGB, HSV), mix memory, lock, boxed std colours, undelete, contrast pick | E4 |
| `w_color.c` | `DoGrabPixel` | 2451 | missing | pick a colour from the screen (eyedropper) | E4, W10-36 |
| `w_color.c` | `doGrab` | 2496 | missing | pick a colour from the screen (eyedropper) | E4, W10-36 |
| `w_color.c` | `xyToWindowCmap` | 2558 | not needed | X colormap cell allocation and private colormaps |  |
| `w_cursor.c` | `init_cursor` | 44 | partial | egui cursor icons: default and crosshair only; xfig uses a cursor per mode | S4-4 |
| `w_cursor.c` | `recolor_cursors` | 90 | partial | egui cursor icons: default and crosshair only; xfig uses a cursor per mode | S4-4 |
| `w_cursor.c` | `reset_cursor` | 116 | partial | egui cursor icons: default and crosshair only; xfig uses a cursor per mode | S4-4 |
| `w_cursor.c` | `set_temp_cursor` | 126 | partial | egui cursor icons: default and crosshair only; xfig uses a cursor per mode | S4-4 |
| `w_cursor.c` | `set_cursor` | 136 | partial | egui cursor icons: default and crosshair only; xfig uses a cursor per mode | S4-4 |
| `w_digitize.c` | `popup_digitize_panel` | 76 | missing | Digitize panel: tablet input against a scanned figure | D11 |
| `w_digitize.c` | `create_digitize_panel` | 100 | missing | Digitize panel: tablet input against a scanned figure | D11 |
| `w_digitize.c` | `digitize_panel_done` | 372 | missing | Digitize panel: tablet input against a scanned figure | D11 |
| `w_digitize.c` | `digitize_panel_cancel` | 382 | missing | Digitize panel: tablet input against a scanned figure | D11 |
| `w_digitize.c` | `update_example` | 412 | missing | Digitize panel: tablet input against a scanned figure | D11 |
| `w_digitize.c` | `switch_file_mode` | 433 | missing | Digitize panel: tablet input against a scanned figure | D11 |
| `w_dir.c` | `FileSelected` | 144 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `DirSelected` | 178 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `ShowHidden` | 190 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `update_file_export_dir` | 203 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `GoHome` | 212 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `SetDir` | 240 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `parseuserpath` | 271 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `create_dirinfo` | 288 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `free_list` | 541 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `freelists` | 551 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `SPComp` | 566 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `replace_entry` | 572 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `extend_list` | 586 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `MakeFileList` | 612 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `ParentDir` | 755 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `DoChangeDir` | 771 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `Rescan` | 824 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `NewList` | 871 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `IsDirectory` | 888 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `MakeFullPath` | 912 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `wild_match` | 963 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_dir.c` | `star` | 1007 | replaced | directory and file lists of xfig's own file panel are the native dialogs (`rfd`; decided 2026-10-14); in-app Open panel with preview is W10-12 | W10-12 |
| `w_drawprim.c` | `init_font` | 98 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `pw_xfttext` | 132 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `pw_text` | 163 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `textsize` | 213 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `makegc` | 234 | not needed | X graphics contexts, Xlib wrappers, allocation checks |  |
| `w_drawprim.c` | `init_gc` | 267 | not needed | X graphics contexts, Xlib wrappers, allocation checks |  |
| `w_drawprim.c` | `init_fill_gc` | 304 | replaced | fill patterns: fig2dev `scene/pattern.rs` (D12) |  |
| `w_drawprim.c` | `init_fill_pm` | 719 | replaced | fill patterns: fig2dev `scene/pattern.rs` (D12) |  |
| `w_drawprim.c` | `pw_vector` | 802 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `pw_curve` | 816 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `pw_point` | 862 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `pw_arcbox` | 884 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `pw_lines` | 948 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `erase_box` | 1021 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `set_clip_window` | 1028 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `set_zoomed_clip_window` | 1043 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `reset_clip_window` | 1049 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `set_fill_gc` | 1055 | replaced | fill patterns: fig2dev `scene/pattern.rs` (D12) |  |
| `w_drawprim.c` | `set_line_stuff` | 1132 | replaced | drawing primitives and text of the X window: the renderer (`rustfig_render`, fig2dev scene) and egui painter |  |
| `w_drawprim.c` | `rescale_pattern` | 1251 | replaced | fill patterns: fig2dev `scene/pattern.rs` (D12) |  |
| `w_drawprim.c` | `scale_pattern` | 1279 | replaced | fill patterns: fig2dev `scene/pattern.rs` (D12) |  |
| `w_drawprim.c` | `zXDrawLines` | 1357 | not needed | X graphics contexts, Xlib wrappers, allocation checks |  |
| `w_drawprim.c` | `zXFillPolygon` | 1369 | not needed | X graphics contexts, Xlib wrappers, allocation checks |  |
| `w_drawprim.c` | `convert_sh` | 1384 | not needed | X graphics contexts, Xlib wrappers, allocation checks |  |
| `w_drawprim.c` | `chkalloc` | 1395 | not needed | X graphics contexts, Xlib wrappers, allocation checks |  |
| `w_export.c` | `export_panel_dismiss` | 191 | replaced | export dialog `dialogs::export_dialog` with the language menu in xfig's order; command `file.export` |  |
| `w_export.c` | `export_panel_cancel` | 211 | replaced | export dialog `dialogs::export_dialog` with the language menu in xfig's order; command `file.export` |  |
| `w_export.c` | `exp_getxyoff` | 220 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `do_export` | 241 | replaced | export dialog `dialogs::export_dialog` with the language menu in xfig's order; command `file.export` |  |
| `w_export.c` | `preview_select` | 380 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `orient_select` | 393 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `just_select` | 409 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `papersize_select` | 421 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `multiple_select` | 438 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `overlap_select` | 474 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `set_postscript_pdf_options` | 489 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `lang_select` | 521 | replaced | export dialog `dialogs::export_dialog` with the language menu in xfig's order; command `file.export` |  |
| `w_export.c` | `set_export_mask` | 550 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `smooth_select` | 577 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `background_select` | 590 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `transp_select` | 618 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `export_grid_minor_select` | 643 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `export_grid_major_select` | 669 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `manage_optional` | 696 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `export_update_figure_size` | 856 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `fit_page` | 877 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `get_magnif` | 925 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `update_mag` | 941 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `update_figure_size` | 952 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `get_quality` | 969 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `popup_export_panel` | 985 | replaced | export dialog `dialogs::export_dialog` with the language menu in xfig's order; command `file.export` |  |
| `w_export.c` | `exp_xoff_unit_select` | 1064 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `exp_yoff_unit_select` | 1073 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `toggle_hpgl_pcl_switch` | 1083 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `toggle_hpgl_font` | 1106 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `toggle_pdf_pagemode` | 1129 | missing | Export panel options: Fit to Page and figure size, orientation, justification, paper size, multiple pages and overlap, offsets with units, background, grid, smoothing, transparent colour, PS/PDF options, HPGL/PCL options, quality, magnification | W10-4, W10-10 |
| `w_export.c` | `create_export_panel` | 1151 | replaced | export dialog `dialogs::export_dialog` with the language menu in xfig's order; command `file.export` |  |
| `w_export.c` | `update_def_filename` | 2021 | replaced | export dialog `dialogs::export_dialog` with the language menu in xfig's order; command `file.export` |  |
| `w_file.c` | `file_panel_dismiss` | 199 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `file_getxyoff` | 230 | missing | Figure Offset X/Y fields with units in the Open/Merge panel | W10-12 |
| `w_file.c` | `merge_request` | 246 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `do_merge` | 260 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `load_request` | 317 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `do_load` | 331 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `new_xfig_request` | 402 | not needed | PROPOSAL: "New xfig..." starts a second xfig process on a chosen file; RustFig has one window per process (window-parity.md records it as low value) |  |
| `w_file.c` | `save_request` | 454 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `set_curfiledir_from_savepath` | 477 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `do_save` | 525 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `renamefile` | 663 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `query_save` | 675 | replaced | Save changes? query before New/Open/Quit: `dialogs::unsaved_dialog` |  |
| `w_file.c` | `cancel_request` | 696 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `file_panel_cancel` | 710 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `popup_saveas_panel` | 725 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `popup_open_panel` | 731 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `popup_merge_panel` | 737 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `popup_file_panel` | 743 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `file_xoff_unit_select` | 851 | missing | Figure Offset X/Y fields with units in the Open/Merge panel | W10-12 |
| `w_file.c` | `file_yoff_unit_select` | 860 | missing | Figure Offset X/Y fields with units in the Open/Merge panel | W10-12 |
| `w_file.c` | `create_file_panel` | 868 | replaced | Open, Save As and Merge use the native dialogs; commands `file.open`, `file.save`, `file.merge`, `file.rename` |  |
| `w_file.c` | `check_cancel` | 1334 | missing | preview of the selected figure in the file panel, with size and comments | W10-12 |
| `w_file.c` | `clear_preview` | 1345 | missing | preview of the selected figure in the file panel, with size and comments | W10-12 |
| `w_file.c` | `file_preview_stop` | 1368 | missing | preview of the selected figure in the file panel, with size and comments | W10-12 |
| `w_file.c` | `preview_figure` | 1379 | missing | preview of the selected figure in the file panel, with size and comments | W10-12 |
| `w_file.c` | `save_user_colors` | 1677 | not needed | saving user colours around a preview (the preview will use its own document) |  |
| `w_file.c` | `save_nuser_colors` | 1701 | not needed | saving user colours around a preview (the preview will use its own document) |  |
| `w_file.c` | `restore_user_colors` | 1722 | not needed | saving user colours around a preview (the preview will use its own document) |  |
| `w_file.c` | `restore_nuser_colors` | 1776 | not needed | saving user colours around a preview (the preview will use its own document) |  |
| `w_fontpanel.c` | `init_fontmenu` | 85 | replaced | font menus: the Text Font attribute pop-up `attributes` (Attr::Font); the font chooser is E11 | E11 |
| `w_fontpanel.c` | `setup_fontmenu` | 278 | replaced | font menus: the Text Font attribute pop-up `attributes` (Attr::Font); the font chooser is E11 | E11 |
| `w_fontpanel.c` | `fontpane_popup` | 343 | replaced | font menus: the Text Font attribute pop-up `attributes` (Attr::Font); the font chooser is E11 | E11 |
| `w_fontpanel.c` | `fontpane_select` | 372 | replaced | font menus: the Text Font attribute pop-up `attributes` (Attr::Font); the font chooser is E11 | E11 |
| `w_fontpanel.c` | `fontpane_cancel` | 403 | replaced | font menus: the Text Font attribute pop-up `attributes` (Attr::Font); the font chooser is E11 | E11 |
| `w_fontpanel.c` | `fontpane_swap` | 409 | replaced | font menus: the Text Font attribute pop-up `attributes` (Attr::Font); the font chooser is E11 | E11 |
| `w_grid.c` | `init_grid` | 45 | ported | grid drawing in the canvas, `view.toggle_grid`, `view.set_grid` |  |
| `w_grid.c` | `setup_grid` | 62 | ported | grid drawing in the canvas, `view.toggle_grid`, `view.set_grid` |  |
| `w_help.c` | `launch_refman` | 48 | missing | Help menu entries that open the reference manual, PDF manuals, how-to and man page, and external viewer launching | W10-2 |
| `w_help.c` | `launch_refpdf_en` | 65 | missing | Help menu entries that open the reference manual, PDF manuals, how-to and man page, and external viewer launching | W10-2 |
| `w_help.c` | `launch_refpdf_jp` | 75 | missing | Help menu entries that open the reference manual, PDF manuals, how-to and man page, and external viewer launching | W10-2 |
| `w_help.c` | `launch_howto` | 86 | missing | Help menu entries that open the reference manual, PDF manuals, how-to and man page, and external viewer launching | W10-2 |
| `w_help.c` | `launch_man` | 96 | missing | Help menu entries that open the reference manual, PDF manuals, how-to and man page, and external viewer launching | W10-2 |
| `w_help.c` | `launch_viewer` | 106 | missing | Help menu entries that open the reference manual, PDF manuals, how-to and man page, and external viewer launching | W10-2 |
| `w_help.c` | `check_docfile` | 131 | missing | Help menu entries that open the reference manual, PDF manuals, how-to and man page, and external viewer launching | W10-2 |
| `w_help.c` | `help_ok` | 140 | missing | Help menu entries that open the reference manual, PDF manuals, how-to and man page, and external viewer launching | W10-2 |
| `w_help.c` | `launch_about` | 149 | ported | About dialog (`dialogs::about_dialog`) |  |
| `w_i18n.c` | `setlocale` | 401 | not needed | Xt locale initialisation; RustFig's localisation is F4 | F4 |
| `w_icons.c` | `populate_icons_big` | 2156 | ported | all 107 bitmaps verbatim in `ui-egui/xfig_icons.rs`, vector versions in `vector_icons.rs`/`mode_icons.rs` (W9) |  |
| `w_icons.c` | `populate_icons_small` | 2245 | ported | all 107 bitmaps verbatim in `ui-egui/xfig_icons.rs`, vector versions in `vector_icons.rs`/`mode_icons.rs` (W9) |  |
| `w_icons.c` | `setup_icons_small` | 3258 | ported | all 107 bitmaps verbatim in `ui-egui/xfig_icons.rs`, vector versions in `vector_icons.rs`/`mode_icons.rs` (W9) |  |
| `w_icons.c` | `setup_icons_big` | 3265 | ported | all 107 bitmaps verbatim in `ui-egui/xfig_icons.rs`, vector versions in `vector_icons.rs`/`mode_icons.rs` (W9) |  |
| `w_indpanel.c` | `init_ind_panel` | 565 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `add_ind_actions` | 781 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `note_state` | 787 | ported | `cmd/indicator.rs` `indicator.set_update_flag`, `indicator.update_flags`; `ui-egui/attributes.rs` update boxes (Update mode only) |  |
| `w_indpanel.c` | `tog_all_update` | 804 | ported | `cmd/indicator.rs` `indicator.set_update_flag`, `indicator.update_flags`; `ui-egui/attributes.rs` update boxes (Update mode only) |  |
| `w_indpanel.c` | `set_all_update` | 822 | ported | `cmd/indicator.rs` `indicator.set_update_flag`, `indicator.update_flags`; `ui-egui/attributes.rs` update boxes (Update mode only) |  |
| `w_indpanel.c` | `clr_all_update` | 840 | ported | `cmd/indicator.rs` `indicator.set_update_flag`, `indicator.update_flags`; `ui-egui/attributes.rs` update boxes (Update mode only) |  |
| `w_indpanel.c` | `manage_update_buts` | 856 | ported | `cmd/indicator.rs` `indicator.set_update_flag`, `indicator.update_flags`; `ui-egui/attributes.rs` update boxes (Update mode only) |  |
| `w_indpanel.c` | `unmanage_update_buts` | 865 | ported | `cmd/indicator.rs` `indicator.set_update_flag`, `indicator.update_flags`; `ui-egui/attributes.rs` update boxes (Update mode only) |  |
| `w_indpanel.c` | `setup_ind_panel` | 874 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `generate_choice_pixmaps` | 973 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `update_indpanel` | 990 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `sel_ind_but` | 1074 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `update_string_pixmap` | 1111 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `update_choice_pixmap` | 1132 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `arrowsize_panel_dismiss` | 1170 | partial | Arrow Size panel: width and length only; absolute/multiple-of-width modes and thickness are not there | W10-31 |
| `w_indpanel.c` | `arrowsize_panel_cancel` | 1177 | partial | Arrow Size panel: width and length only; absolute/multiple-of-width modes and thickness are not there | W10-31 |
| `w_indpanel.c` | `arrowsize_panel_set` | 1186 | partial | Arrow Size panel: width and length only; absolute/multiple-of-width modes and thickness are not there | W10-31 |
| `w_indpanel.c` | `set_arrow_size_state` | 1213 | partial | Arrow Size panel: width and length only; absolute/multiple-of-width modes and thickness are not there | W10-31 |
| `w_indpanel.c` | `popup_arrowsize_panel` | 1281 | partial | Arrow Size panel: width and length only; absolute/multiple-of-width modes and thickness are not there | W10-31 |
| `w_indpanel.c` | `choice_panel_dismiss` | 1539 | replaced | choice pop-ups (line style, join, cap, arrow, fill, justification, ...) are egui pop-ups; live values and hover preview are W10-34, W10-35 |  |
| `w_indpanel.c` | `choice_panel_cancel` | 1546 | replaced | choice pop-ups (line style, join, cap, arrow, fill, justification, ...) are egui pop-ups; live values and hover preview are W10-34, W10-35 |  |
| `w_indpanel.c` | `choice_panel_set` | 1554 | replaced | choice pop-ups (line style, join, cap, arrow, fill, justification, ...) are egui pop-ups; live values and hover preview are W10-34, W10-35 |  |
| `w_indpanel.c` | `popup_choice_panel` | 1587 | replaced | choice pop-ups (line style, join, cap, arrow, fill, justification, ...) are egui pop-ups; live values and hover preview are W10-34, W10-35 |  |
| `w_indpanel.c` | `flags_panel_dismiss` | 1852 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `flags_panel_cancel` | 1859 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `flags_panel_set` | 1867 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `hidden_text_select` | 1888 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `rigid_text_select` | 1901 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `special_text_select` | 1914 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `popup_flags_panel` | 1926 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `nval_panel_dismiss` | 2106 | partial | number pop-ups (zoom, depth, angle, sides, step, ...) exist; Integer zoom check box and Return-sets are W10-32 | W10-32 |
| `w_indpanel.c` | `toggle_int_zoom` | 2115 | partial | number pop-ups (zoom, depth, angle, sides, step, ...) exist; Integer zoom check box and Return-sets are W10-32 | W10-32 |
| `w_indpanel.c` | `nval_panel_cancel` | 2128 | partial | number pop-ups (zoom, depth, angle, sides, step, ...) exist; Integer zoom check box and Return-sets are W10-32 | W10-32 |
| `w_indpanel.c` | `nval_panel_set` | 2135 | partial | number pop-ups (zoom, depth, angle, sides, step, ...) exist; Integer zoom check box and Return-sets are W10-32 | W10-32 |
| `w_indpanel.c` | `popup_nval_panel` | 2153 | partial | number pop-ups (zoom, depth, angle, sides, step, ...) exist; Integer zoom check box and Return-sets are W10-32 | W10-32 |
| `w_indpanel.c` | `popup_dimline_panel` | 2399 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `line_color_select` | 2878 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `box_color_select` | 2893 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `text_color_select` | 2908 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `set_fixed_text` | 2923 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `set_actual_text` | 2939 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `draw_cur_dimline` | 2955 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `dimline_style_select` | 3060 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `left_arrow_type_select` | 3075 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `right_arrow_type_select` | 3089 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `dimline_font_popup` | 3105 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `dimline_set_font_image` | 3113 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `dimline_panel_preview` | 3126 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `dimline_panel_ok` | 3177 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `dimline_panel_cancel` | 3187 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `get_dimline_values` | 3200 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `update_current_settings` | 3227 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `dec_choice` | 3239 | replaced | choice pop-ups (line style, join, cap, arrow, fill, justification, ...) are egui pop-ups; live values and hover preview are W10-34, W10-35 |  |
| `w_indpanel.c` | `inc_choice` | 3247 | replaced | choice pop-ups (line style, join, cap, arrow, fill, justification, ...) are egui pop-ups; live values and hover preview are W10-34, W10-35 |  |
| `w_indpanel.c` | `show_arrowmode` | 3257 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_arrowtype` | 3286 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_arrowsize` | 3302 | partial | Arrow Size panel: width and length only; absolute/multiple-of-width modes and thickness are not there | W10-31 |
| `w_indpanel.c` | `dec_arrowsize` | 3310 | partial | Arrow Size panel: width and length only; absolute/multiple-of-width modes and thickness are not there | W10-31 |
| `w_indpanel.c` | `show_arrowsize` | 3318 | partial | Arrow Size panel: width and length only; absolute/multiple-of-width modes and thickness are not there | W10-31 |
| `w_indpanel.c` | `inc_dimline` | 3354 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `dec_dimline` | 3362 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `show_dimline` | 3370 | missing | dimension-line attribute panel and indicator button (style, arrows, font, colours, preview) | D10 |
| `w_indpanel.c` | `dec_linewidth` | 3449 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_linewidth` | 3456 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_linewidth` | 3463 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_tangnormlen` | 3537 | ported | Tangent/Norm Length button (`ui-egui/attributes.rs`, `Indicator::tangent_length`, 0 to 20, steps of 0.1) | |
| `w_indpanel.c` | `dec_tangnormlen` | 3544 | ported | Tangent/Norm Length button (`ui-egui/attributes.rs`, `Indicator::tangent_length`, 0 to 20, steps of 0.1) | |
| `w_indpanel.c` | `show_tangnormlen` | 3551 | ported | Tangent/Norm Length button (`ui-egui/attributes.rs`, `Indicator::tangent_length`, 0 to 20, steps of 0.1) | |
| `w_indpanel.c` | `show_anglegeom` | 3569 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `w_indpanel.c` | `show_arctype` | 3621 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_joinstyle` | 3638 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_capstyle` | 3657 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_linestyle` | 3676 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_valign` | 3713 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_halign` | 3744 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_gridmode` | 3775 | partial | Grid Mode button with its choices; the four isometric grids are W10-33 | W10-33 |
| `w_indpanel.c` | `show_pointposn` | 3799 | ported | Point Posn choices act on every point (`Indicator::snap_point`); a mode never hides the button (xfig's forced on/off only greys it) |  |
| `w_indpanel.c` | `show_linkmode` | 3824 | ported | Smart Links indicator button off/move/slide (`ui-egui/attributes.rs`) |  |
| `w_indpanel.c` | `show_textjust` | 3843 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `dec_boxradius` | 3862 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_boxradius` | 3869 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_boxradius` | 3878 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `darken_fill` | 3914 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `lighten_fill` | 3926 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_fillstyle` | 3938 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `recolor_fillstyles` | 3977 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_flags` | 4018 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `dec_flags` | 4026 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `show_flags` | 4034 | replaced | Text Flags pop-up (hidden, rigid, special, LaTeX) |  |
| `w_indpanel.c` | `inc_font` | 4063 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `dec_font` | 4073 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_font` | 4083 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `popup_fonts` | 4134 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_font_return` | 4143 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_fontsize` | 4156 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `dec_fontsize` | 4170 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_fontsize` | 4184 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_elltextangle` | 4199 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `dec_elltextangle` | 4211 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_elltextangle` | 4222 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_rotnangle` | 4242 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `dec_rotnangle` | 4262 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_rotnangle` | 4282 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_rotnangle_0` | 4290 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `set_and_show_rotnangle` | 4324 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_numsides` | 4342 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `dec_numsides` | 4349 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_numsides` | 4356 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_numcopies` | 4373 | missing | Number of copies, and X and Y copies for array copy | D10 |
| `w_indpanel.c` | `dec_numcopies` | 4380 | missing | Number of copies, and X and Y copies for array copy | D10 |
| `w_indpanel.c` | `show_numcopies` | 4387 | missing | Number of copies, and X and Y copies for array copy | D10 |
| `w_indpanel.c` | `inc_numxcopies` | 4404 | missing | Number of copies, and X and Y copies for array copy | D10 |
| `w_indpanel.c` | `dec_numxcopies` | 4411 | missing | Number of copies, and X and Y copies for array copy | D10 |
| `w_indpanel.c` | `show_numxcopies` | 4418 | missing | Number of copies, and X and Y copies for array copy | D10 |
| `w_indpanel.c` | `inc_numycopies` | 4436 | missing | Number of copies, and X and Y copies for array copy | D10 |
| `w_indpanel.c` | `dec_numycopies` | 4443 | missing | Number of copies, and X and Y copies for array copy | D10 |
| `w_indpanel.c` | `show_numycopies` | 4450 | missing | Number of copies, and X and Y copies for array copy | D10 |
| `w_indpanel.c` | `inc_zoom` | 4476 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `dec_zoom` | 4506 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `zoom_focus` | 4523 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `inc_zoom_centered` | 4547 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `dec_zoom_centered` | 4554 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `wheel_inc_zoom` | 4562 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `wheel_dec_zoom` | 4584 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `fit_zoom` | 4606 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `show_zoom` | 4671 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `inc_depth` | 4720 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `dec_depth` | 4727 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_depth` | 4734 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `inc_textstep` | 4752 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `dec_textstep` | 4769 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `show_textstep` | 4787 | ported | indicator buttons with step and show for: arrow mode and type, line width, arc type, join, cap and line style, vertical and horizontal align, text justification, box radius, fill style, font, font size, text angle, rotation angle, number of sides, depth, text step |  |
| `w_indpanel.c` | `zoom_to_fit` | 4805 | ported | zoom button: step, centred step, wheel, fit, focus (`view.zoom_*`, `view.set_scale`); Integer zoom is W10-32 |  |
| `w_indpanel.c` | `make_pulldown_menu_images` | 4821 | replaced | attribute (indicator) panel `ui-egui/attributes` and `engine::Indicator`: construction, update, pixmaps and button selection |  |
| `w_indpanel.c` | `tog_selective_update` | 4846 | ported | `cmd/indicator.rs` `indicator.set_update_flag`, `indicator.update_flags`; `ui-egui/attributes.rs` update boxes (Update mode only) |  |
| `w_intersect.c` | `is_point_on_segment` | 42 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `insert_isect` | 49 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `fget_line_from_points` | 65 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `boxes_overlap` | 78 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `do_circle_ellipse_intersect` | 96 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `circle_ellipse_intersect` | 200 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `do_circle_circle` | 214 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `circle_circle_intersect` | 313 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `non_ortho_ellipse_ellipse_intersect` | 334 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `ortho_ellipse_ellipse_intersect` | 571 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_ellipse_ellipse_handler` | 745 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `do_intersect_ellipse_polyline` | 779 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_ellipse_polyline_handler` | 984 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_ellipse_spline_handler` | 996 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `delete_text_bounding_box` | 1005 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `build_text_bounding_box` | 1014 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_ellipse_text_handler` | 1044 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_ellipse_arc_handler` | 1052 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_polyline_polyline_handler` | 1085 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_polyline_spline_handler` | 1162 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_polyline_text_handler` | 1171 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_polyline_arc_handler` | 1179 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_spline_spline_handler` | 1191 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_spline_text_handler` | 1200 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_spline_arc_handler` | 1209 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_text_text_handler` | 1218 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_text_arc_handler` | 1231 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_arc_arc_handler` | 1242 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_ellipse_handler` | 1260 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_polyline_handler` | 1286 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_spline_handler` | 1312 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_text_handler` | 1338 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `intersect_arc_handler` | 1364 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_intersect.c` | `snap_intersect_handler` | 1390 | ported | `engine/snap.rs` (W4): intersection of every pair of object kinds, on flattened outlines instead of the analytic ellipse solvers |  |
| `w_keyboard.c` | `next_keyboard_history` | 62 | missing | type coordinates of the next point in a keyboard panel (with history) | D10 |
| `w_keyboard.c` | `prior_keyboard_history` | 74 | missing | type coordinates of the next point in a keyboard panel (with history) | D10 |
| `w_keyboard.c` | `ignore_keyboard_input` | 87 | missing | type coordinates of the next point in a keyboard panel (with history) | D10 |
| `w_keyboard.c` | `handle_keyboard_input` | 94 | missing | type coordinates of the next point in a keyboard panel (with history) | D10 |
| `w_keyboard.c` | `create_keyboard_panel` | 417 | missing | type coordinates of the next point in a keyboard panel (with history) | D10 |
| `w_keyboard.c` | `popup_keyboard_panel` | 487 | missing | type coordinates of the next point in a keyboard panel (with history) | D10 |
| `w_keyboard.c` | `popdown_keyboard_panel` | 512 | missing | type coordinates of the next point in a keyboard panel (with history) | D10 |
| `w_layers.c` | `reset_layers` | 102 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `reset_depths` | 111 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `init_depth_panel` | 120 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `setup_depth_panel` | 335 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `update_layers` | 366 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `draw_layer_buttons` | 391 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `draw_layer_button` | 409 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `layer_exposed` | 456 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `calculate_pressed_depth` | 478 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `set_depth_to_layer` | 495 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `toggle_layer` | 508 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `sweep_layer` | 545 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `leave_layer` | 585 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `all_active` | 592 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `all_inactive` | 614 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `toggle_all` | 636 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `switch_layer_mode` | 653 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `any_active_in_compound` | 697 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `update_layerpanel` | 728 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `toggle_show_depths` | 776 | ported | depth panel `panels::depth_panel`, `depth.*` commands: layers, counts, toggle, sweep, all on/off, show depths |  |
| `w_layers.c` | `save_active_layers` | 804 | not needed | saving and swapping layer state around print/export (RustFig passes the layer list to the export) |  |
| `w_layers.c` | `save_depths` | 814 | not needed | saving and swapping layer state around print/export (RustFig passes the layer list to the export) |  |
| `w_layers.c` | `save_counts` | 824 | not needed | saving and swapping layer state around print/export (RustFig passes the layer list to the export) |  |
| `w_layers.c` | `save_counts_and_clear` | 832 | not needed | saving and swapping layer state around print/export (RustFig passes the layer list to the export) |  |
| `w_layers.c` | `restore_active_layers` | 839 | not needed | saving and swapping layer state around print/export (RustFig passes the layer list to the export) |  |
| `w_layers.c` | `restore_depths` | 849 | not needed | saving and swapping layer state around print/export (RustFig passes the layer list to the export) |  |
| `w_layers.c` | `restore_counts` | 861 | not needed | saving and swapping layer state around print/export (RustFig passes the layer list to the export) |  |
| `w_layers.c` | `swap_depths` | 871 | not needed | saving and swapping layer state around print/export (RustFig passes the layer list to the export) |  |
| `w_layers.c` | `swap_counts` | 894 | not needed | saving and swapping layer state around print/export (RustFig passes the layer list to the export) |  |
| `w_library.c` | `SPComp` | 190 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `LRComp` | 198 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `put_selected_request` | 208 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `library_stop` | 222 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `library_dismiss` | 232 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `library_cancel` | 239 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `load_library` | 299 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `put_object_sel` | 389 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `load_lib_obj` | 417 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `NewObjectSel` | 499 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `popup_library_panel` | 540 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `create_library_panel` | 584 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `make_library_menu` | 1067 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `sel_view` | 1107 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `change_icon_size` | 1157 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `set_preview_name` | 1184 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `set_cur_obj_name` | 1192 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `erase_pixmap` | 1199 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `PutLibraryEntry` | 1206 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `ScanLibraryDirectory` | 1245 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `MakeLibrary` | 1358 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `MakeObjectLibrary` | 1422 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `MakeLibraryFileList` | 1559 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `libraryStatus` | 1610 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `preview_libobj` | 1623 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `update_preview` | 1741 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `set_comments` | 1749 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `sel_item_icon` | 1762 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `unsel_icon` | 1801 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `sel_icon` | 1810 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_library.c` | `copy_icon_to_preview` | 1825 | missing | Library panel: browse a library directory, previews, place objects | D6 |
| `w_listwidget.c` | `SelectHome` | 182 | replaced | list navigation keys of the file panel: the native dialogs |  |
| `w_listwidget.c` | `SelectEnd` | 192 | replaced | list navigation keys of the file panel: the native dialogs |  |
| `w_listwidget.c` | `SelectNext` | 205 | replaced | list navigation keys of the file panel: the native dialogs |  |
| `w_listwidget.c` | `SelectPrev` | 229 | replaced | list navigation keys of the file panel: the native dialogs |  |
| `w_listwidget.c` | `SelectLeft` | 252 | replaced | list navigation keys of the file panel: the native dialogs |  |
| `w_listwidget.c` | `SelectRight` | 275 | replaced | list navigation keys of the file panel: the native dialogs |  |
| `w_listwidget.c` | `FigListFinishUp` | 297 | replaced | list navigation keys of the file panel: the native dialogs |  |
| `w_modepanel.c` | `stub_enter_mode_btn` | 311 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `init_mode_panel` | 383 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `free_mode_pixmaps` | 442 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `setup_mode_panel` | 459 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `update_modepanel` | 499 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `sel_mode_but` | 522 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `force_positioning` | 601 | ported | Point Posn choices act on every point (`Indicator::snap_point`); a mode never hides the button (xfig's forced on/off only greys it) |  |
| `w_modepanel.c` | `force_nopositioning` | 608 | ported | Point Posn choices act on every point (`Indicator::snap_point`); a mode never hides the button (xfig's forced on/off only greys it) |  |
| `w_modepanel.c` | `force_anglegeom` | 615 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `w_modepanel.c` | `force_noanglegeom` | 621 | ported | `engine/anglegeom.rs` (`constrain`, `latex_endpoint`), `Engine::angle_constrained` / `angle_anchor` (tools), Angle Geom button in `ui-egui/attributes` |  |
| `w_modepanel.c` | `turn_on` | 627 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `turn_on_current` | 633 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `turn_off_current` | 639 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `change_mode` | 649 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_circle_ellipse_byradius_drawing_selected` | 665 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_circle_ellipse_bydiameter_drawing_selected` | 671 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_box_drawing_selected` | 677 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_arcbox_drawing_selected` | 683 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_poly_drawing_selected` | 689 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_line_drawing_selected` | 695 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_regpoly_drawing_selected` | 701 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_picobj_drawing_selected` | 707 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_text_drawing_selected` | 713 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_arc_drawing_selected` | 719 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_cl_spline_drawing_selected` | 725 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_spline_drawing_selected` | 731 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_cl_intspline_drawing_selected` | 737 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_intspline_drawing_selected` | 743 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_align_selected` | 749 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_compound_selected` | 755 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_break_selected` | 761 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_join_split_selected` | 767 | ported | mode button `JOIN_SPLIT` in the editing group | D2 |
| `w_modepanel.c` | `stub_open_compound_selected` | 773 | ported | mode button `OPEN_COMP` in the editing group | D1 |
| `w_modepanel.c` | `stub_scale_selected` | 779 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_point_adding_selected` | 785 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_delete_point_selected` | 791 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_move_selected` | 797 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_popup_library` | 802 | missing | Library mode button (greyed out) | D6 |
| `w_modepanel.c` | `stub_move_point_selected` | 809 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_delete_selected` | 815 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_copy_selected` | 821 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_rotate_cw_selected` | 827 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_rotate_ccw_selected` | 833 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_flip_ud_selected` | 839 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_flip_lr_selected` | 845 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_convert_selected` | 851 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_chop_selected` | 857 | ported | Chop mode button | D3 |
| `w_modepanel.c` | `stub_arrow_head_selected` | 863 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_edit_item_selected` | 869 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_update_selected` | 875 | ported | mode panel (`panels::mode_panel`), mode switching (`Engine::set_tool`), per-mode masks and attribute sets (`attributes_for`) |  |
| `w_modepanel.c` | `stub_tangent_selected` | 881 | ported | Tangent mode button, key `n` | |
| `w_modepanel.c` | `stub_anglemeas_selected` | 887 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `w_modepanel.c` | `stub_lenmeas_selected` | 893 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `w_modepanel.c` | `stub_areameas_selected` | 899 | ported | `cmd/measure.rs`, `tools/measure.rs` (`tool.angle_measure`, `tool.length_measure`, `tool.area_measure`) | |
| `w_mousefun.c` | `init_mousefun` | 69 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `setup_mousefun` | 97 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `update_mousepanel` | 111 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `reset_mousefun` | 123 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `mouse_title` | 185 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `resize_mousefun` | 197 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `set_mousefun` | 204 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `draw_mousefun_kbd` | 218 | missing | mouse-function text for the keyboard coordinate panel | D10 |
| `w_mousefun.c` | `clear_mousefun_kbd` | 236 | missing | mouse-function text for the keyboard coordinate panel | D10 |
| `w_mousefun.c` | `draw_mousefun_mode` | 248 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `draw_mousefun_ind` | 254 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `draw_mousefun_unitbox` | 260 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `draw_mousefun_topruler` | 266 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `draw_mousefun_sideruler` | 281 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `draw_shift_mousefun_canvas` | 296 | partial | the indicator's Shift and Ctrl variants while the key is held (Shift: del point, dimension line, ...; Ctrl: Zoom area, Pan to origin, Unzoom; Ctrl+Shift: More approx, Cycle shapes, More interp) | S4-7 |
| `w_mousefun.c` | `draw_shift_mousefun_canvas2` | 302 | partial | the indicator's Shift and Ctrl variants while the key is held (Shift: del point, dimension line, ...; Ctrl: Zoom area, Pan to origin, Unzoom; Ctrl+Shift: More approx, Cycle shapes, More interp) | S4-7 |
| `w_mousefun.c` | `draw_mousefun_canvas` | 314 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `draw_mousefun_msg` | 320 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `draw_mousefun` | 334 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `draw_mousefn2` | 347 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `notused_middle` | 366 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `clear_middle` | 376 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `notused_right` | 386 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `clear_right` | 396 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `clear_mousefun` | 406 | ported | mouse-function indicator `panels::mouse_indicator`, `Engine::mouse_hints` (layout and wording, S2) |  |
| `w_mousefun.c` | `init_kbd_actions` | 426 | missing | mouse-function text for the keyboard coordinate panel | D10 |
| `w_msgpanel.c` | `init_msg` | 92 | ported | message panel `panels::message_panel`, `Engine::set_status` |  |
| `w_msgpanel.c` | `setup_msg` | 113 | ported | message panel `panels::message_panel`, `Engine::set_status` |  |
| `w_msgpanel.c` | `put_msg` | 125 | ported | message panel `panels::message_panel`, `Engine::set_status` |  |
| `w_msgpanel.c` | `boxsize_msg` | 154 | missing | lengths, sizes and angles shown while drawing (Show lengths), dimension strings | D10 |
| `w_msgpanel.c` | `boxsize_scale_msg` | 240 | missing | lengths, sizes and angles shown while drawing (Show lengths), dimension strings | D10 |
| `w_msgpanel.c` | `erase_box_lengths` | 264 | missing | lengths, sizes and angles shown while drawing (Show lengths), dimension strings | D10 |
| `w_msgpanel.c` | `erase_lengths` | 276 | missing | lengths, sizes and angles shown while drawing (Show lengths), dimension strings | D10 |
| `w_msgpanel.c` | `length_msg` | 295 | missing | lengths, sizes and angles shown while drawing (Show lengths), dimension strings | D10 |
| `w_msgpanel.c` | `altlength_msg` | 306 | missing | lengths, sizes and angles shown while drawing (Show lengths), dimension strings | D10 |
| `w_msgpanel.c` | `length_msg2` | 483 | missing | lengths, sizes and angles shown while drawing (Show lengths), dimension strings | D10 |
| `w_msgpanel.c` | `arc_msg` | 521 | missing | lengths, sizes and angles shown while drawing (Show lengths), dimension strings | D10 |
| `w_msgpanel.c` | `lenmeas_msg` | 550 | ported | `cmd/measure.rs` (`length_message`, `area_message`, `dimension_string`) | |
| `w_msgpanel.c` | `areameas_msg` | 566 | ported | `cmd/measure.rs` (`length_message`, `area_message`, `dimension_string`) | |
| `w_msgpanel.c` | `free_filename` | 582 | ported | file name message and the message history (`file_msg`, `dialogs::messages_dialog`; filter and copy W10-29) |  |
| `w_msgpanel.c` | `file_msg` | 594 | ported | file name message and the message history (`file_msg`, `dialogs::messages_dialog`; filter and copy W10-29) |  |
| `w_msgpanel.c` | `clear_file_message` | 670 | ported | file name message and the message history (`file_msg`, `dialogs::messages_dialog`; filter and copy W10-29) |  |
| `w_msgpanel.c` | `file_msg_panel_dismiss` | 704 | ported | file name message and the message history (`file_msg`, `dialogs::messages_dialog`; filter and copy W10-29) |  |
| `w_msgpanel.c` | `popup_file_msg` | 713 | ported | file name message and the message history (`file_msg`, `dialogs::messages_dialog`; filter and copy W10-29) |  |
| `w_msgpanel.c` | `make_dimension_string` | 802 | missing | lengths, sizes and angles shown while drawing (Show lengths), dimension strings | D10 |
| `w_print.c` | `print_panel_dismiss` | 153 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `do_print` | 164 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `fit_page` | 225 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `get_magnif` | 273 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `update_mag` | 289 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `update_figure_size` | 300 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `do_print_batch` | 315 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `do_clear_batch` | 381 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `update_batch_count` | 399 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `orient_select` | 418 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `just_select` | 434 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `papersize_select` | 447 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `multiple_select` | 463 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `overlap_select` | 498 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `background_select` | 514 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `print_grid_minor_select` | 542 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `print_grid_major_select` | 567 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `print_command_select` | 590 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `printer_select` | 601 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `print_update_figure_size` | 617 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `popup_print_panel` | 637 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `create_print_panel` | 678 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `switch_print_layers` | 1279 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `switch_bound` | 1322 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `parse_printcap` | 1346 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_print.c` | `make_layer_choice` | 1431 | missing | Print panel: printer and command, batch list, page layout, magnification, layers (modern replacement planned: preview and PDF to the OS print dialog) | B5 |
| `w_rottext.c` | `my_strtok` | 60 | not needed | rotated text through X bitmaps (the renderer rotates glyph outlines) |  |
| `w_rottext.c` | `XRotDrawString` | 113 | not needed | rotated text through X bitmaps (the renderer rotates glyph outlines) |  |
| `w_rottext.c` | `XRotDrawImageString` | 130 | not needed | rotated text through X bitmaps (the renderer rotates glyph outlines) |  |
| `w_rottext.c` | `XRotDrawHorizontalString` | 146 | not needed | rotated text through X bitmaps (the renderer rotates glyph outlines) |  |
| `w_rulers.c` | `redisplay_rulers` | 309 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `setup_rulers` | 316 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `reset_rulers` | 323 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `set_rulermark` | 331 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `erase_rulermark` | 340 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `init_unitbox` | 349 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `update_rulerpanel` | 392 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `unit_panel_dismiss` | 418 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `unit_panel_cancel` | 425 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `unit_panel_set` | 434 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `set_unit_indicator` | 494 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `fig_unit_select` | 533 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `fig_scale_select` | 555 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `rul_unit_select` | 577 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `popup_unit_panel` | 616 | ported | units box and Unit/Scale panel (`app.unit_panel`, `view.set_scale`; details W10-24) | W10-24 |
| `w_rulers.c` | `topruler_selected` | 863 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `erase_toprulermark` | 929 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `set_toprulermark` | 936 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `topruler_exposed` | 948 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `redisplay_topruler` | 957 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `init_topruler` | 963 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `setup_topruler` | 988 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `resize_topruler` | 1038 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `get_skip_prec` | 1051 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `reset_topruler` | 1101 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `sideruler_selected` | 1194 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `sideruler_exposed` | 1261 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `init_sideruler` | 1271 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `redisplay_sideruler` | 1295 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `setup_sideruler` | 1300 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `resize_sideruler` | 1356 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `reset_sideruler` | 1364 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `erase_siderulermark` | 1441 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_rulers.c` | `set_siderulermark` | 1453 | ported | `ui-egui/rulers.rs`: top and side rulers with ticks, labels, cursor marker, panning (W6) |  |
| `w_setup.c` | `setup_sizes` | 45 | not needed | widget size computation for the X layout (egui lays out) |  |
| `w_snap.c` | `snap_rotate_vector` | 70 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `point_to_line` | 86 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `get_line_from_points` | 109 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_polyline_endpoint_handler` | 126 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_polyline_focus_handler` | 149 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_polyline_midpoint_handler` | 181 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `do_snap_polyline_normal` | 213 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_polyline_normal_handler` | 263 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_ellipse_focus_ellipse_handler` | 276 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `check_alignment` | 315 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_ellipse_normal_ellipse_handler` | 339 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `circle_normal_handler` | 446 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_ellipse_normal_circle_handler` | 487 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `circle_tangent_handler` | 507 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_ellipse_tangent_circle_handler` | 552 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_ellipse_tangent_ellipse_handler` | 567 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_ellipse_endpoint_handler` | 708 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_polyline_handler` | 745 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_spline_handler` | 785 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_text_handler` | 862 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `is_point_on_arc` | 874 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_arc_handler` | 909 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_ellipse_handler` | 1020 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_handler` | 1109 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_process` | 1158 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_hold` | 1191 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_release` | 1199 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_endpoint` | 1208 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_midpoint` | 1224 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_nearest` | 1240 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_focus` | 1252 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_diameter` | 1272 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_intersect` | 1297 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_normal` | 1309 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `snap_tangent` | 1341 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_snap.c` | `init_snap_panel` | 1376 | ported | `engine/snap.rs` and `snap.*` commands (W4); curves snap on flattened outlines |  |
| `w_srchrepl.c` | `compare_string` | 157 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `do_replace` | 167 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `replace_text_in_compound` | 192 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `do_update` | 252 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `found_text_panel_dismiss` | 268 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `show_search_result` | 279 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `show_search_msg` | 311 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `show_text_object` | 325 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `search_and_replace_text` | 343 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `search_text_in_compound` | 379 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `search_panel_dismiss` | 415 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `popup_search_panel` | 427 | ported | `edit.search_replace` and `dialogs::search_dialog` (search, replace, replace all, case); extras W10-26 |  |
| `w_srchrepl.c` | `popup_spell_check_panel` | 633 | missing | spell check panel | F5 |
| `w_srchrepl.c` | `spell_panel_dismiss` | 810 | missing | spell check panel | F5 |
| `w_srchrepl.c` | `spell_check` | 822 | missing | spell check panel | F5 |
| `w_srchrepl.c` | `write_text_from_compound` | 904 | missing | spell check panel | F5 |
| `w_srchrepl.c` | `spell_select_word` | 919 | missing | spell check panel | F5 |
| `w_srchrepl.c` | `spell_correct_word` | 940 | missing | spell check panel | F5 |
| `w_srchrepl.c` | `show_spell_msg` | 955 | missing | spell check panel | F5 |
| `w_style.c` | `construct_style` | 103 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `set_style` | 139 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `print_indent` | 165 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `print_style` | 174 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `print_style_family` | 203 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `print_family_set` | 220 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `get_nstyle_line` | 234 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `trim` | 252 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `parse_style` | 273 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `parse_family_style` | 326 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `parse_family_set` | 353 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `load_family_set` | 366 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `save_family_set` | 385 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `add_family_to_family_set` | 401 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `add_style_to_family` | 423 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `delete_family_from_family_set` | 458 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `delete_style_from_family` | 501 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `build_family_text` | 543 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `build_style_text` | 555 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `style_update` | 571 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `family_select` | 589 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `style_select` | 603 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `add_family` | 619 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `delete_family` | 634 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `add_style` | 650 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `delete_style` | 673 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `save_style` | 690 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `load_style` | 698 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `confirm_close_style` | 708 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `close_style` | 722 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `init_manage_style_panel` | 766 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `add_style_actions` | 992 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `setup_manage_style_panel` | 1001 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_style.c` | `popup_manage_style_panel` | 1019 | missing | Manage Styles: named attribute sets stored in a file (families, styles, load/save) | E6 |
| `w_util.c` | `app_flush` | 191 | replaced | yes/no/cancel queries: `dialogs::unsaved_dialog` and egui dialogs |  |
| `w_util.c` | `accept_yes` | 199 | replaced | yes/no/cancel queries: `dialogs::unsaved_dialog` and egui dialogs |  |
| `w_util.c` | `accept_no` | 207 | replaced | yes/no/cancel queries: `dialogs::unsaved_dialog` and egui dialogs |  |
| `w_util.c` | `accept_cancel` | 215 | replaced | yes/no/cancel queries: `dialogs::unsaved_dialog` and egui dialogs |  |
| `w_util.c` | `accept_part` | 223 | replaced | yes/no/cancel queries: `dialogs::unsaved_dialog` and egui dialogs |  |
| `w_util.c` | `accept_all` | 231 | replaced | yes/no/cancel queries: `dialogs::unsaved_dialog` and egui dialogs |  |
| `w_util.c` | `popup_query` | 239 | replaced | yes/no/cancel queries: `dialogs::unsaved_dialog` and egui dialogs |  |
| `w_util.c` | `make_pulldown_menu` | 376 | replaced | pull-down and colour pop-up menus, colour buttons: egui combo boxes and the colour pop-up (`attributes`) |  |
| `w_util.c` | `CvtStringToFloat` | 402 | not needed | Xt resource converters |  |
| `w_util.c` | `CvtIntToFloat` | 417 | not needed | Xt resource converters |  |
| `w_util.c` | `fix_converters` | 429 | not needed | Xt resource converters |  |
| `w_util.c` | `cancel_color` | 437 | replaced | pull-down and colour pop-up menus, colour buttons: egui combo boxes and the colour pop-up (`attributes`) |  |
| `w_util.c` | `make_color_popup_menu` | 444 | replaced | pull-down and colour pop-up menus, colour buttons: egui combo boxes and the colour pop-up (`attributes`) |  |
| `w_util.c` | `set_color_name` | 581 | replaced | pull-down and colour pop-up menus, colour buttons: egui combo boxes and the colour pop-up (`attributes`) |  |
| `w_util.c` | `set_but_col` | 598 | replaced | pull-down and colour pop-up menus, colour buttons: egui combo boxes and the colour pop-up (`attributes`) |  |
| `w_util.c` | `inc_flt_spinner` | 612 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `dec_flt_spinner` | 636 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `inc_int_spinner` | 661 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `dec_int_spinner` | 685 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `start_spin_timer` | 718 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `stop_spin_timer` | 733 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `auto_spin` | 742 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `MakeIntSpinnerEntry` | 758 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `MakeFloatSpinnerEntry` | 769 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `free_spinner` | 776 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `MakeSpinnerEntry` | 805 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `validate_int` | 958 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `spinner_up_down` | 1011 | replaced | spinners: egui `DragValue` and number pop-ups |  |
| `w_util.c` | `file_msg_add_grab` | 1031 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `get_pointer_win_xy` | 1040 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `get_pointer_root_xy` | 1053 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `check_action_on` | 1067 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `process_pending` | 1088 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `create_bitmaps` | 1097 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `panel_set_value` | 1150 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `panel_get_value` | 1162 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `panel_set_int` | 1174 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `panel_set_float` | 1184 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `CreateCheckbutton` | 1194 | replaced | check boxes: egui `checkbox` |  |
| `w_util.c` | `toggle_checkbutton` | 1277 | replaced | check boxes: egui `checkbox` |  |
| `w_util.c` | `update_wm_title` | 1319 | ported | window title shows the file name and unsaved mark (`update_wm_title`) |  |
| `w_util.c` | `check_for_resize` | 1331 | not needed | resize handling of the X layout |  |
| `w_util.c` | `resize_all` | 1350 | not needed | resize handling of the X layout |  |
| `w_util.c` | `XSyncOn` | 1470 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `XSyncOff` | 1476 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `make_grid_options` | 1483 | replaced | grid choice menus and grid string parsing: `attributes` Grid Mode; export grid fields W10-4 |  |
| `w_util.c` | `reset_grid_menus` | 1599 | replaced | grid choice menus and grid string parsing: `attributes` Grid Mode; export grid fields W10-4 |  |
| `w_util.c` | `convert_gridstr` | 1677 | replaced | grid choice menus and grid string parsing: `attributes` Grid Mode; export grid fields W10-4 |  |
| `w_util.c` | `getxcolor` | 1738 | not needed | X pointer grabs, event pumping, X bitmaps, panel value getters/setters, X sync |  |
| `w_util.c` | `splash_screen` | 1750 | not needed | PROPOSAL: splash screen at start (xfig shows a picture for a few seconds; RustFig starts straight into the window, the About box has the credits) |  |
| `w_util.c` | `clear_splash` | 1959 | not needed | PROPOSAL: splash screen at start (xfig shows a picture for a few seconds; RustFig starts straight into the window, the About box has the credits) |  |
| `w_util.c` | `InstallScroll` | 1979 | not needed | X scroll-wheel handlers on widgets |  |
| `w_util.c` | `InstallScrollParent` | 1996 | not needed | X scroll-wheel handlers on widgets |  |
| `w_util.c` | `_installscroll` | 2002 | not needed | X scroll-wheel handlers on widgets |  |
| `w_zoom.c` | `zoom_selected` | 66 | missing | zoom box: drag a rectangle to zoom into it | E3 |
| `w_zoom.c` | `unzoom` | 101 | partial | Unzoom (back to the previous view) exists as zoom out only; the previous-view semantics are D11 | D11 |
| `w_zoom.c` | `my_box` | 108 | missing | zoom box: drag a rectangle to zoom into it | E3 |
| `w_zoom.c` | `elastic_mybox` | 117 | missing | zoom box: drag a rectangle to zoom into it | E3 |
| `w_zoom.c` | `init_zoombox_drawing` | 123 | missing | zoom box: drag a rectangle to zoom into it | E3 |
| `w_zoom.c` | `do_zoom` | 150 | missing | zoom box: drag a rectangle to zoom into it | E3 |
| `w_zoom.c` | `cancel_zoom` | 188 | missing | zoom box: drag a rectangle to zoom into it | E3 |
| `xfigmain.c` | `xfigmain` | 624 | ported | as main.c: `engine::cmdline` | S3 |
| `xfigmain.c` | `setup_visual` | 1626 | not needed | as main.c: X visual and colormap set-up |  |
| `xfigmain.c` | `make_cut_buf_name` | 1744 | not needed | as main.c: X cut-buffer property name (the clipboard is in memory, E7 for the system clipboard) |  |
| `xfigmain.c` | `check_resource_ranges` | 1769 | replaced | as main.c: resource values are range-checked when set (`engine::prefs`) |  |
| `xfigmain.c` | `set_icon_geom` | 1798 | not needed | as main.c: X window-manager icon geometry |  |
| `xfigmain.c` | `set_max_image_colors` | 1818 | not needed | as main.c: X visual and colormap set-up |  |
| `xfigmain.c` | `parse_canvas_colors` | 1848 | not needed | as main.c: X visual and colormap set-up |  |
| `xfigmain.c` | `set_xpm_icon` | 1901 | replaced | as main.c: window icon from `app_icon.rs` |  |
| `xfigmain.c` | `resize_canvas` | 1959 | not needed | as main.c: canvas size follows the egui layout |  |
| `xfigmain.c` | `get_pointer_mapping` | 2012 | replaced | as main.c: mouse button mapping: `MouseScheme` and the left+right = middle chord (`input.rs`, W11-1) |  |
| `xfigmain.c` | `set_autorefresh` | 2037 | not needed | as main.c: PROPOSAL: Autorefresh mode (View menu in xfig) redraws the canvas at an interval to clean up X11 drawing leftovers; egui redraws every frame. Task W10-3 records the decision | W10-3 |
| `xfigmain.c` | `cancel_autorefresh` | 2075 | not needed | as main.c: PROPOSAL: Autorefresh mode (View menu in xfig) redraws the canvas at an interval to clean up X11 drawing leftovers; egui redraws every frame. Task W10-3 records the decision | W10-3 |
| `xfigmain.c` | `toggle_refresh_mode` | 2092 | not needed | as main.c: PROPOSAL: Autorefresh mode (View menu in xfig) redraws the canvas at an interval to clean up X11 drawing leftovers; egui redraws every frame. Task W10-3 records the decision | W10-3 |
| `xfigmain.c` | `check_refresh` | 2109 | not needed | as main.c: PROPOSAL: Autorefresh mode (View menu in xfig) redraws the canvas at an interval to clean up X11 drawing leftovers; egui redraws every frame. Task W10-3 records the decision | W10-3 |
