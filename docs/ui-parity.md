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
| Picture | `tool.picture` (PNG/JPEG/GIF/BMP/TIFF/PNM) | 🟡 no EPS/PDF/XPM |
| Text | `tool.text` | 🟡 no character map |
| Library object | — | ❌ |

## Editing modes
| xfig mode | RustFig | |
|---|---|---|
| Glue / break compound | `tool.glue`, `tool.break` | ✅ |
| Open compound | — | ❌ |
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
| Join/split, chop, tangent/normal | — | ❌ |
| Angle / length / area measure | `object.measure` (command only) | 🟡 |

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
| Indicator panel (line, colours, fill, depth, arrows, caps, joins, fonts, justification, angle, radius, sides, rotation, grid, positioning) | bottom panel | 🟡 no latex/manhattan positioning |
| Rulers | — | ❌ |
| Mouse-function panel | top strip, one column per button | ✅ |

## Main window (xfig screen map)
| xfig | RustFig | Plan |
|---|---|---|
| Main menus File, Edit, View, Snap, Help | ✅ in xfig's order with xfig labels (Object and Tools as extras); Snap has point positioning and grid, object snaps missing | W4 |
| Current file name | ✅ next to the menus, marks unsaved changes | — |
| Message panel | ✅ under the menus; click for the history | — |
| Mouse-function indicator | ✅ each button's current function per tool and state, checked against the tools by a test | — |
| Three-button mouse semantics | ✅ xfig's per-mode functions (default) or a modern scheme (View › xfig mouse buttons); Alt+click = middle, Ctrl+click = right. Missing: dimension lines, rotate/flip anchor, array copy, align to canvas | D |
| Units box | 🟡 shows the units, opens units and page settings; xfig's scale panel missing | W6 |
| Top and side rulers | ❌ | W6 |
| Depth panel: checkboxes, drag-toggle, All On/Off/Toggle, Gray/Blank | ✅ with object counts; hidden depths can't be picked; export of active layers only | — |
| Attribute panel: graphical buttons, per-mode attributes | 🟡 all attributes always shown as plain widgets | W5 |
| xfig accelerators (Meta-key) | 🟡 platform shortcuts and xfig mode letters | W7 |

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First checklist. |
| 2026-10-13 | Added the main-window section (plan phase W). |
| 2026-10-13 | W1: classic layout, message panel, mouse indicator, units box. |
| 2026-10-14 | W2: three-button mouse semantics and the modern scheme. |
| 2026-10-14 | W3: depth panel. |
