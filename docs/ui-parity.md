# UI parity checklist

**Status:** 2026-10-22 (E4, W10-36: colour picker and editor, recent colours, eyedropper); 2026-10-22 (E10: text editing); 2026-10-22 (S4-3, S4-8, S4-9: Digitize, New window, Autorefresh); 2026-10-22 (W10-15, W10-17 to W10-19, W10-21: live edit panel, compound panel, point table, pictorial menus); 2026-10-22 (W10-16, W10-20: picture edit panel); 2026-10-22 (D5: measure modes); 2026-10-22 (W10-28, W10-2: bundled manual); 2026-10-22 (D4: Tangent mode and its length button). ✅ done · 🟡 partial · ❌ missing

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
| Picture edit panel | Reread, Browse/Relink, drop a file, type, size, colours, transparency, thumbnail, width/length, rotation, h/w ratios, Shrink/Expand/Original/Scale %, Edit Image, XBM pen colour (W10-16, W10-20) | ✅ |
| Picture | `tool.picture` (xfig's types: EPS/PS, PDF, GIF, JPEG, PCX, PNG, PPM, TIFF, XBM, XPM, `.gz`/`.Z`; also BMP) | ✅ |
| Text | `tool.text`: caret by click, selection (mouse, Shift+arrows), word jumps, clipboard, IME (E10) | ✅ character map (E5: View > Character map, inserts at the caret; Symbol and Dingbats tables) |
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
| Edit | `tool.edit` + edit dialog: live (every change shows at once; Done is one undo step, Cancel undoes all), compound rows (corners, size, depth range, Collapse Depths, texts), point table (insert, delete, add, paste, X-spline shape factors), pictorial menus for line style, cap, join and arrow type, no tints for black/white fills | ✅ |
| Flip up/down, left/right | `tool.flip_vertical`, `tool.flip_horizontal` | ✅ |
| Rotate | `tool.rotate_ccw`, `tool.rotate_cw` | ✅ |
| Convert | `tool.convert` | ✅ |
| Open/close | `tool.open_close` | ✅ |
| Add/remove arrows | `tool.arrow` | ✅ |
| Join/split | `tool.join_split`, `object.join`, `object.split` | ✅ |
| Chop | `tool.chop`, `object.chop` | ✅ (D3, D15) |
| Trim, split and extend (RustFig's own, next to Chop, so the later editing buttons move one slot; original vector icon) | `tool.trim` (`w`), commands `object.trim`, `object.split_at`, `object.extend`; left trims, middle splits, right extends; Shift locates | ✅ (D16) |
| Tangent/normal | `tool.tangent` (`n`), `object.tangent_line`; Tangent/Norm Length button | ✅ (D4) |
| Angle / length / area measure | `tool.angle_measure` (Ctrl+G), `tool.length_measure` (Ctrl+N), `tool.area_measure` (Ctrl+M); `object.measure_*` | ✅ (D5) |

The window-by-window comparison with the real xfig (what each window offers and does, rule: same or more functionality) is in [window-parity.md](window-parity.md) (W10).

## Menus and panels
| xfig | RustFig | |
|---|---|---|
| File: New/Open/Merge/Save/Save As/Export/Exit | ✅ | ✅ |
| File: Print | — | ❌ |
| File: Digitize... (Alt+Z): prefix, starting number, suffix, new file or append; RustFig saves the points of the selected lines, optionally calibrated (S4-8) | ✅ | ✅ |
| File: New window... (xfig's New xfig, native only; S4-9) | ✅ | ✅ |
| View: Autorefresh mode (reloads a file changed on disk, asks with unsaved edits; native only; S4-9) | ✅ | ✅ |
| Edit: Undo (multi-level), Paste, Delete All, Search/Replace | ✅ | ✅ |
| Edit: Spell check, Global settings | Global settings ✅ (W10-23: xfig's rows with Ok/Cancel); Spell check F5 | 🟡 |
| View: zoom, fit, pan to origin, page border, portrait/landscape | ✅ | ✅ |
| View: depth manager | right panel | ✅ |
| View: character map | `app.character_map`, Ctrl+Shift+M | ✅ |
| View: Manage Styles… (Ctrl+Y): style families as in xfig's panel (Family and Style lists, Add, Delete, Set), kept in the settings | ✅ E6 (`crates/ui-egui/src/styles.rs`); xfig's `~/.xfigrcstyle` file is replaced by settings.toml |
| Indicator panel (line, colours, fill, depth, arrows, caps, joins, fonts, justification, angle, radius, sides, rotation, grid, positioning, arc type, text step, text flags, smart links) | xfig's buttons, per mode | 🟡 Angle Geom done (D7); no copy counts |
| S4 editing functions | Ctrl+click zoom box, pan to origin and unzoom (E3), Ctrl+Shift+click shape factor (S4-6), `Engine::mouse_hints_for(shift, ctrl)`; Shift+click locate object (S4-1), Paste attached to the pointer (S4-2), system clipboard for the cut buffer and images (E7), a cursor per mode (S4-4), Convert box/arc-box (S4-5) | ✅ |
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
| Indicator: Num Copies (rotate), Num X / Num Y Copies (copy), Dimension line (Polyline; settings window as `popup_dimline_panel`); typed-coordinate window (Edit > Enter point…, Ctrl+K); View > Show line lengths | ✅ (D10) | D |
| Three-button mouse semantics | ✅ xfig's per-mode functions, the only scheme (W16); Alt+click = middle, Ctrl+click = right. Dimension lines (Shift+left), rotate/flip anchor and centre (right button), array copy (middle while placing), align to canvas (middle) work (D10); the indicator buttons and the dimension-line panel too | D |
| Units box | ✅ "1cm = 1.00cm" in the ruler corner; left: pan to origin, right: Unit/Scale panel with Cancel/Set, Show fractions, User defined unit and scale (W10-24) | — |
| Top and side rulers | ✅ ported from `w_rulers.c` | — |
| Depth panel: checkboxes, drag-toggle, All On/Off/Toggle, Gray/Blank | ✅ with object counts; hidden depths can't be picked; export of active layers only | — |
| Update Control: update box on every attribute button in Update mode, Set/Clear/Toggle all buttons, flags honoured by Update, update region, lasso and update settings (W10-1) | ✅ | — |
| Attribute panel: graphical buttons, per-mode attributes | ✅ xfig's buttons and order, per-mode masks, choice panels; middle/right buttons step values; W19: xfig's labels and layout, each button narrowed to what its label and value need, wraps to a second row | — |
| xfig accelerators (Meta-key) | ✅ xfig keymap (on by default): Meta = Alt/Option menu keys, physical Ctrl mode keys, xfig's mode letters; menus show "Meta-N"; platform shortcuts stay | — |
| Colour picker and editor (xfig's colour panel `w_color.c`) | ✅ (E4b adds Pick from screen: macOS NSColorSampler, Windows live pick, Linux portal PickColor, greyed out on the web) Edit › Colours…, the colour pop-up's recent colours, More… and Eyedropper (E4, W10-36): hue/saturation square, RGB/HSV, hex and X11 names, user colours (add, change, delete, remove unused, keep under a name); mix memory and lock not built | — |
| Preferences (xfig's resources, recent files, user colours) | ✅ `settings.toml`; Edit › Global settings › xfig resources… | — |
| Icons at HiDPI | ✅ every icon and picture is drawn as a vector at the screen's density (xfig's bitmaps optional); application icon | — |
| Font menus (`w_fontpanel.c`): Text Font pop-up and the edit panel's font menu | ✅ font chooser (E11): PostScript/LaTeX switch, every name in its own font, search, size field, sample preview, current font marked | — |
| Theme and scale | ✅ Global settings: Theme System/Light/Dark, UI scale 75 to 200 %, kept in the preferences; the paper stays white (E9) | — |
| 3D (bevelled) buttons, Global setting "Button style: 3D / Flat" | ✅ raised light/dark edges and soft shading, sunken when pressed or selected; mode, depth, snap, attribute and top-strip buttons; `app.button_style`, saved in the preferences (W11) | — |
| Mode panel never clipped | ✅ two columns and every row fit at the default window size; buttons shrink to 26 pt before the panel scrolls (W11) | — |
| Snap Mode box | ✅ a chooser (None and the Snap menu's modes), HOLD tag while held, Hold checked and Release enabled only when held (additions, W11) | — |
| File-name box | ✅ click to type; Enter renames the file (`file.rename`), asks before replacing; unsaved figure: sets the name for Save (addition, W11) | — |
| Mouse Buttons box | ✅ compact, labels next to the buttons; no keyboard label (decided 2026-10-21) | — |
| Rulers | ✅ left up/left, right down/right on release (Shift x5), middle and Alt+right drag, wheel = buttons 4/5, left-drag added; pan limit at 0 not copied (decided 2026-10-21) | — |
| xfig's other key and mouse bindings (menu letters, panel keys, wheel bindings, compose keys) | ✅ audited in [keys-parity.md](keys-parity.md): each works, is replaced, or waits for a named task | — |

## Command-line options (S3)

`rustfig [options] [file]` takes every option of xfig's `xfig.man.in` (`crates/engine/src/cmdline.rs`, table `OPTIONS`; tests in the same file). Options with an equivalent set an xfig resource for **that run only** (the preferences file is not changed); a value follows the option and is consumed whatever it looks like. Options may be abbreviated to a prefix as in Xt: an exact name wins, otherwise the first option of xfig's table that starts with the argument (`-me` is `-metric`, `-spec` is `-specialtext`). An unknown option, a missing value or a second file name stops with a message, as in xfig. Resources are also read from `~/.Xresources` and `~/.Xdefaults` (`Fig.name: value`, `xfig*name: value`) and fill in what the settings file lacks; `-xrm 'Fig.name: value'` works. Not done: `~/.xfigrc` import, an `xfig` launcher name, `-scale_factor` for `-update`.

| Option | Effect in RustFig |
|---|---|
| `-metric`, `-centimeters`; `-inches`, `-imperial` | `inches` resource: units of a new drawing |
| `-landscape`, `-Landscape`; `-portrait`, `-Portrait` | `landscape`: orientation of a new drawing |
| `-paper_size SIZE`, `-magnification PCT`, `-flushleft`/`-center`, `-multiple`/`-single` | paper, export magnification, justification and page mode of a new drawing |
| `-specialtext`, `-hiddentext`, `-rigidtext`, `-latexfonts`, `-left`/`-right` | start text attributes of the indicator |
| `-startfontsize` (`-startFontSize`), `-startlinewidth`, `-startfillstyle`, `-starttextstep`, `-startpsFont`, `-startlatexFont` | start attributes |
| `-startgridmode N`, `-startposnmode N` | grid and point-positioning mode |
| `-startarrowtype`, `-startarrowthick`, `-startarrowwidth`, `-startarrowlength` | arrow defaults |
| `-zoom`, `-userscale`, `-userunit`, `-showpageborder`/`-dontshowpageborder` | view and unit box |
| `-showballoons`/`-dontshowballoons`, `-balloon_delay`, `-showdepthmanager`/`-dontshowdepthmanager` | UI panels |
| `-exportLanguage`, `-export_margin`, `-correct_font_size` | export panel defaults |
| `-write_bak`/`-nowrite_bak` | applied: Save keeps the old file as NAME.bak |
| `-geometry WxH+X+Y` | window size and position |
| `-update FILE...` | read each file, write it back in the current Fig format, keep `FILE.bak`, quit (no window) |
| `-help`, `-h`, `-v`, `-V` | usage / version, then quit |
| `--control PORT`, `--background` | RustFig's own (control channel; open behind other windows) |
| `-allownegcoords`, `-autorefresh`, `-axislines`, `-but_per_row`, `-cbg`, `-cfg`, `-crosshair`, `-encoding`, `-flipvisualhints`, `-freehand_resolution`, `-grid_color`, `-grid_unit`, `-icon_view`/`-list_view`, `-image_editor`, `-jpeg_quality`, `-keyFile`, `-library_dir`, `-nooverlap`/`-overlap`, `-pageborder`, `-pheight`, `-pwidth`, `-rulerthick`, `-scale_factor`, `-showallbuttons`, `-showaxislines`, `-showlengths`, `-shownums`, `-smallicons`, `-smooth_factor`, `-spellcheckcommand`, `-track`/`-notrack`, `-transparent_color` | accepted and kept as the resource of that name; takes effect when its feature lands (the plan task is in the resource's status) |
| `-library_icon_size`, `-startgridtype`, `-write_v40`, `-inputStyle`, `-gslib` | accepted; note "not supported yet" |
| `-visual`, `-depth`, `-dontswitchcmap`, `-installowncmap`, `-monochrome`, `-buttonFont`, `-normalFont`, `-internalBW`, `-debug`, `-ghostscript`, `-iconGeometry`, `-nosplash`/`-splash`, `-tablet`, `-spinner_delay`, `-spinner_rate`, `-max_image_colors`, `-international`/`-nointernational` | X11 specifics: accepted, ignored with a note on stderr |
| `-display`, `-iconic`, `-name`, `-title`, `-bg`/`-background`, `-fg`/`-foreground`, `-bd`/`-bordercolor`, `-bw`/`-borderwidth`, `-fn`/`-font`, `-rv`/`-reverse`/`+rv`, `-synchronous`/`+synchronous`, `-selectionTimeout`, `-xnllanguage`, `-xtsessionID` | X toolkit options: accepted, ignored with a note |

## Revision history
| Date | Note |
|---|---|
| 2026-10-22 | E7 done: the cut buffer on the system clipboard (Fig text + pictures), Paste takes a figure from another window or an image. |
| 2026-10-22 | E4, W10-36 done: colour picker and editor (Edit > Colours…), recent colours, eyedropper, `color.*` commands. |
| 2026-10-22 | E4b done: Pick from screen in the Colours window (macOS, Windows, Linux portal; web greyed out). |
| 2026-10-22 | S4-3, S4-8, S4-9: Digitize window, New window, Autorefresh mode; relative picture names follow Save As. |
| 2026-10-22 | S3: every option of `xfig.man.in` accepted (table above): per-run overrides of the preferences, abbreviations as in Xt, X toolkit options mapped or ignored with a note, `-update`, `-geometry`, `~/.Xdefaults` resources. |
| 2026-10-22 | W10-28, W10-2: Help > RustFig Manual opens the bundled manual window (topic list, search, rendered page); Help letter `x` opens it, `c` the command list. |
| 2026-10-22 | D14: the Smart Links button also on Move point, Scale, Flip, Rotate and Align. |
| 2026-10-22 | D8: Smart Links indicator button (off/move/slide). |
| 2026-10-22 | D16: Trim, split and extend mode (`tool.trim`, button next to Chop). |
| 2026-10-22 | D15: Chop mode's Shift+left/middle/right (Trim, Split at point, Extend). |
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
