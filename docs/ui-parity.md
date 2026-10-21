# UI parity checklist

**Status:** 2026-10-14. ✅ done · 🟡 partial · ❌ missing

## Drawing modes
| xfig mode | RustFig | |
|---|---|---|
| Circle by radius / diameter | `tool.circle_radius`, `tool.circle_diameter` | ✅ |
| Ellipse by radii / diameter | `tool.ellipse_radii`, `tool.ellipse_diameter` | ✅ |
| Closed/open approximated spline | `tool.closed_spline`, `tool.spline` | ✅ |
| Closed/open interpolated spline | `tool.closed_interp_spline`, `tool.interp_spline` | ✅ |
| Closed/open X-spline | `tool.closed_xspline`, `tool.xspline` | ✅ |
| Polygon, polyline | `tool.polygon`, `tool.polyline` | ✅ |
| Box, arc-box | `tool.box`, `tool.arc_box` | ✅ |
| Regular polygon | `tool.regular_polygon` | ✅ |
| Arc | `tool.arc` (three clicks) | ✅ |
| Picture | `tool.picture` (xfig's types: EPS/PS, PDF, GIF, JPEG, PCX, PNG, PPM, TIFF, XBM, XPM, `.gz`/`.Z`; also BMP) | ✅ |
| Text | `tool.text` | 🟡 no character map |
| Library object | — | ❌ |

## Editing modes
| xfig mode | RustFig | |
|---|---|---|
| Glue / break compound | `tool.glue`, `tool.break` | ✅ |
| Open compound | `tool.open_compound`, `compound.open`, `compound.close`, `compound.close_all`, Close Compound popup | ✅ |
| Scale | `tool.scale` | ✅ |
| Align | `tool.align`, `arrange.align` | ✅ |
| Move / copy | `tool.move`, `tool.copy` | ✅ |
| Move / add / delete point | `tool.move_point`, `tool.add_point`, `tool.delete_point` | ✅ |
| Delete | `tool.delete` | ✅ |
| Update | `tool.update` | ✅ |
| Edit | `tool.edit` + edit dialog | ✅ |
| Flip up/down, left/right | `tool.flip_vertical`, `tool.flip_horizontal` | ✅ |
| Rotate | `tool.rotate_ccw`, `tool.rotate_cw` | ✅ |
| Convert | `tool.convert` | ✅ |
| Open/close | `tool.open_close` | ✅ |
| Add/remove arrows | `tool.arrow` | ✅ |
| Join/split | `tool.join_split`, `object.join`, `object.split` | ✅ |
| Chop | `tool.chop`, `object.chop` | ✅ |
| Tangent/normal | — | ❌ |
| Angle / length / area measure | `object.measure` (command only) | 🟡 |

The window-by-window comparison with the real xfig (what each window offers and does, rule: same or more functionality) is in [window-parity.md](window-parity.md) (W10).

## Menus and panels
| xfig | RustFig | |
|---|---|---|
| File: New/Open/Merge/Save/Save As/Export/Exit | ✅ | ✅ |
| File: Print | — | ❌ |
| Edit: Undo (multi-level), Paste, Delete All, Search/Replace | ✅ | ✅ |
| Edit: Spell check, Global settings | — | ❌ |
| View: zoom, fit, pan to origin, page border, portrait/landscape | ✅ | ✅ |
| View: depth manager | right panel | ✅ |
| View: character map, style manager | — | ❌ |
| Indicator panel (line, colours, fill, depth, arrows, caps, joins, fonts, justification, angle, radius, sides, rotation, grid, positioning, arc type, text step, text flags) | xfig's buttons, per mode | 🟡 no Angle Geom (latex/manhattan), Smart Links, copy counts |
| Object markers in editing modes | per-mode masks as xfig (`u_markers.c`, `w_modepanel.c`) | ✅ |
| Rulers | top and side, xfig's ticks and labels, cursor marker, pan buttons | ✅ |
| Mouse-function panel | top strip, one column per button | ✅ |

## Main window (xfig screen map)
| xfig | RustFig | Plan |
|---|---|---|
| Main menus File, Edit, View, Snap, Help | ✅ classic layout: xfig's menus word for word and in order (`w_cmdpanel.c`), missing entries greyed with their task, RustFig preferences under Edit › Global settings; RustFig's extras after xfig's entries in Edit, View and File and an Object menu between Snap and Help (W17); Snap: Hold, Release, Endpoint, Midpoint, Nearest, Focus, Normal, Tangent, Intersection, Diameter, with the Snap Mode box | — |
| Current file name | ✅ next to the menus, marks unsaved changes | — |
| Message panel | ✅ under the menus; click for the history | — |
| Mouse-function indicator | ✅ xfig's layout and wording, each button's current function per tool and state, checked against the tools by a test | — |
| Three-button mouse semantics | ✅ xfig's per-mode functions, the only scheme (W16); Alt+click = middle, Ctrl+click = right. Missing: dimension lines, rotate/flip anchor, array copy, align to canvas | D |
| Units box | ✅ "1cm = 1.00cm" in the ruler corner; left: pan to origin, right: Unit menu (ruler units, figure unit, scale) | — |
| Top and side rulers | ✅ ported from `w_rulers.c` | — |
| Depth panel: checkboxes, drag-toggle, All On/Off/Toggle, Gray/Blank | ✅ with object counts; hidden depths can't be picked; export of active layers only | — |
| Update Control: update box on every attribute button in Update mode, Set/Clear/Toggle all buttons, flags honoured by Update, update region, lasso and update settings (W10-1) | ✅ | — |
| Attribute panel: graphical buttons, per-mode attributes | ✅ xfig's buttons and order, per-mode masks, choice panels; middle/right buttons step values; W19: xfig's labels and layout, each button narrowed to what its label and value need, wraps to a second row | — |
| xfig accelerators (Meta-key) | ✅ xfig keymap (on by default): Meta = Alt/Option menu keys, physical Ctrl mode keys, xfig's mode letters; menus show "Meta-N"; platform shortcuts stay | — |
| Preferences (xfig's resources, recent files, user colours) | ✅ `settings.toml`; Edit › Global settings › xfig resources… | — |
| Icons at HiDPI | ✅ every icon and picture is drawn as a vector at the screen's density (xfig's bitmaps optional); application icon | — |
| 3D (bevelled) buttons, Global setting "Button style: 3D / Flat" | ✅ raised light/dark edges and soft shading, sunken when pressed or selected; mode, depth, snap, attribute and top-strip buttons; `app.button_style`, saved in the preferences (W11) | — |
| Mode panel never clipped | ✅ two columns and every row fit at the default window size; buttons shrink to 26 pt before the panel scrolls (W11) | — |
| Snap Mode box | ✅ a chooser (None and the Snap menu's modes), HOLD tag while held, Hold checked and Release enabled only when held (additions, W11) | — |
| File-name box | ✅ click to type; Enter renames the file (`file.rename`), asks before replacing; unsaved figure: sets the name for Save (addition, W11) | — |
| Mouse Buttons box | ✅ compact, labels next to the buttons; no keyboard label (decided 2026-10-21) | — |
| Rulers | ✅ left up/left, right down/right on release (Shift x5), middle and Alt+right drag, wheel = buttons 4/5, left-drag added; pan limit at 0 not copied (decided 2026-10-21) | — |
| xfig's other key and mouse bindings (menu letters, panel keys, wheel bindings, compose keys) | ✅ audited in [keys-parity.md](keys-parity.md): each works, is replaced, or waits for a named task | — |

## Revision history
| Date | Note |
|---|---|
| 2026-10-22 | D3: Chop mode added to the editing-modes table (`tool.chop`, `object.chop`). |
| 2026-10-13 | First checklist. |
| 2026-10-13 | Added the main-window section (plan phase W). |
| 2026-10-13 | W1: classic layout, message panel, mouse indicator, units box. |
| 2026-10-14 | W2: three-button mouse semantics and the modern scheme. |
| 2026-10-21 | W19: attribute panel tightened (xfig's layout, buttons narrowed to their content; a short-caption version was tried and reverted). |
| 2026-10-21 | W16, W17: the modern scheme, the layout choice, the attribute-panel switch and the icon choice removed; Object menu; Save All / Save Part query. |
| 2026-10-14 | W3: depth panel. |
| 2026-10-14 | W5: attribute panel. |
| 2026-10-14 | W6: rulers and units box. |
| 2026-10-14 | W4: Snap menu. |
| 2026-10-14 | W7: xfig accelerators. |
| 2026-10-15 | W8 planned: audit of every xfig binding. |
| 2026-10-15 | W9 planned: vector icons. |
| 2026-10-15 | W9: attribute pictures as vectors; arrow types 10–14. |
| 2026-10-15 | W9 done: mode icons traced; application icon. |
| 2026-10-21 | W11: 3D buttons and button style setting, mode panel fits, snap chooser and hold mark, editable file-name box, compact Mouse Buttons box, rulers follow the button events. |
| 2026-10-14 | Mode panel: xfig's icons and order; modes not yet available shown greyed. |
| 2026-10-14 | Main menus cloned from xfig. |
| 2026-10-21 | W10: link to window-parity.md. |
| 2026-10-21 | W13, W14: the mouse-function box shows the new functions on the free buttons (xfig scheme), rubber bands are drawn for the regions, the three-point circle and the align reference. |
| 2026-10-21 | W15: pressing the middle or right button and dragging in a region function (tag, delete, update, move, delete points) draws a freehand lasso; the mouse-function box says "(drag: lasso)" and "release: close lasso". |
