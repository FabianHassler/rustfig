# xfig key and mouse bindings checklist (W8)

**Status:** 2026-10-23 (W21: key notation, both schemes side by side, Help > Keys for the current system). 2026-10-22 (E10: selection, word jumps, clipboard keys while typing). 2026-10-22 (D5: measure keys Ctrl-g, Ctrl-n, Ctrl-m). 2026-10-22 (D4: Tangent mode key `n`). 2026-10-22 (D3: Chop mode key `x`). 2026-10-15: audit complete: every binding works, is replaced by a modern equivalent, or waits for the feature it needs (task named). Every binding xfig defines, from
`app-defaults/Fig` and the 78 translation tables (210 bindings) in xfig's
`src/*.c`, with what RustFig does. X plumbing (window-manager close
messages, expose and resize events, highlight on enter/leave) is left out:
egui does those itself.

Legend: ✅ works as in xfig · 🟡 partly · ❌ missing (task) · ↻ replaced by a
modern equivalent. Meta is Alt (Option on macOS); xfig binds Meta and Alt
alike. RustFig's own platform shortcuts (Ctrl/Cmd+S, …) work alongside: on macOS always (Cmd), on Linux and Windows wherever xfig's Ctrl keys leave them free (W20); a menu entry shows xfig's key, or the platform's where xfig has none.

How keys are written (W21): macOS ⌃ ⌥ ⇧ ⌘ plus a thin space ("⌥ U"), Windows and Linux Ctrl+Alt+Shift+Key; punctuation keys spelled (Quote, Minus, Equals). Menus show xfig's and the system's key side by side ("⌥ U / ⌘ Z"); Help > Keys shows both columns for the current system only. No platform key was added for xfig-keyed commands that have none; both schemes work.

## Accelerators (`Fig*accelerators`)

| Key | xfig action | RustFig | Status |
|---|---|---|---|
| Meta-N | New | `app.new` | ✅ |
| Meta-O | Open | `app.open` | ✅ |
| Meta-M | Merge | `app.merge` | ✅ |
| Meta-Z | Digitize panel | — | ❌ D11 (digitize) |
| Meta-S | Save | `app.save` | ✅ |
| Meta-A | Save As | `app.save_as` | ✅ |
| Shift-Meta-X | Export with current settings | `app.export_now` | ✅ |
| Meta-X | Export panel | `app.export` | ✅ |
| Shift-Meta-P | Print with current settings | — | ✅ `print.quick` |
| Meta-P | Print panel | `app.print` | ✅ |
| Meta-Q | Quit | `app.quit` | ✅ |
| Shift-U | Unit panel | `app.unit_panel` | ✅ |
| Meta-U | Undo | `edit.undo` | ✅ |
| Meta-T | Paste | `edit.paste` | ✅ |
| Meta-I | Search | `app.search` | ✅ |
| Meta-K | Spell check | Spell Check... | ✅ F5 |
| Meta-G | Global settings | `app.global_settings` | ✅ |
| Meta-D | Delete all | `edit.delete_all` | ✅ |
| Ctrl-L, Meta-R | Redraw | `app.redraw` | ✅ |
| Meta-C | Change orientation | `doc.toggle_orientation` | ✅ |
| Shift-Z / Z / Ctrl-Z | Zoom in / out / fit | `view.zoom_in`, `view.zoom_out`, `view.zoom_fit` | ✅ |
| Ctrl-Y | Style families | `app.styles` | ✅ E6 |
| Meta-B | Show page borders | `view.toggle_page_border` | ✅ |
| Meta-Y | Show balloons (tooltips) | `app.toggle_balloons` | ✅ |
| Meta-L | Show line lengths while drawing | Alt+L (xfig keys) | ✅ View > Show line lengths (D10) |
| Meta-F/E/V/H | Pop up the File/Edit/View/Help menu | `menus::XFIG_MENU_KEYS` | ✅ |
| o | Open compound mode (`ModeOpenCompound`) | `tool.open_compound` | ✅ (D1) |
| j | Join/Split mode (`ModeJoinSplit`) | `tool.join_split` | ✅ (D2) |
| x | Chop mode (`ModeChopObject`) | `tool.chop` | ✅ (D3) |
| w | (RustFig's own) Trim, split and extend mode | `tool.trim` | ✅ (D16) |
| n | Tangent/normal mode (`ModeTangent`) | `tool.tangent` | ✅ (D4) |
| Ctrl-g / Ctrl-n / Ctrl-m | Measure angle / length / area (`ModeAnglemeas`, `ModeLenmeas`, `ModeAreameas`) | `tool.angle_measure`, `tool.length_measure`, `tool.area_measure` | ✅ (D5) |

## Letters while a menu is open (`Fig*commands*…*translations`)

| Menu | Letters | Status |
|---|---|---|
| File | n o m d s a e p x; 1–9 recent files | ✅ letters (`XFIG_MENU_LETTERS`; d waits for D11); ❌ recent files E1 |
| Edit | u p t f s d g n | ✅ (t waits for E7, s for D11) |
| View | s r t i o f u p c d n b l a, v v | ✅ for the entries RustFig has (n: balloons); named styles (E6), character map (E5), lengths (D10), auto refresh and vertex numbers (D11) wait for their features |
| Help | x m h a | ✅ x (the manual), c (command reference, RustFig's), a (about); m, h: xfig's man pages and How-To are manual topics, no keys |

## Canvas (`w_canvas.c`)

| Binding | xfig action | Status |
|---|---|---|
| Left/Right/Up/Down | pan by a ruler step (Shift: five) | ✅ |
| Home | pan to the origin | ✅ (`view.pan_origin`) |
| Left/Right/Home/End while typing | move the caret | ✅ |
| Shift+arrows, word jumps (Option on macOS, Ctrl elsewhere), select all, copy/cut/paste while typing (E10, RustFig's own) | selection and system clipboard | ✅ |
| Wheel / Shift+wheel | pan vertically / horizontally | ✅ |
| Ctrl+wheel | zoom | ✅ |
| Ctrl+left/middle/right | zoom area / pan to origin / unzoom | ✅ E3 (Cmd on macOS); Ctrl+Shift+click shape factor ✅ S4-6 |
| Alt+wheel up/down | pop up the drawing/editing mode menus | ✅ (`panels::mode_popup`; closes on a mode, Escape, a click elsewhere or the wheel the other way) |
| Ctrl-K | keyboard coordinate entry panel (`w_keyboard.c`) | ✅ Edit > Enter point… (Ctrl+K; PhysCtrl+K with xfig's keys), D10 |
| Meta (Compose) alone, then two keys | compose characters (`CompKeyDB`, 97 sequences) | ✅ (`rustfig_text::compose`); on macOS Option's own dead keys work too |
| F18, F20 (Sun Paste keys) | paste | ↻ Ctrl/Cmd+V |

## Panels and fields

| Where | Binding | xfig action | Status |
|---|---|---|---|
| Text fields (`*Text*translations`) | Delete, Home, End | delete forward, line start, line end | ✅ (egui) |
| Numeric fields (`resources.c`, `w_indpanel.c`, export, print) | Ctrl-U | Athena's repeat-four prefix | not needed (no equivalent in modern fields) |
| Text fields | Ctrl-X | clear the field | ↻ Ctrl/Cmd+A then Delete (Ctrl/Cmd+X is Cut on every platform today) |
| Text fields | Ctrl-J, Ctrl-M, Return | no new line | ↻ single-line fields |
| Search panel | Return in Find / Replace | search / replace | ✅ |
| Search panel | Escape, Meta-C, Meta-Q | close | ✅ Escape; Meta-C/Q not ported (Meta-Q quits, Meta-C changes orientation in RustFig's global keymap) |
| Export, Global, Document, Commands, About panels | Escape | cancel | ✅ |
| Export panel | Return | export; in Magnification: apply | ✅ (fields apply as they change) |
| Edit popup (`e_edit.c`) | Return / Ctrl+Return / Escape | apply / done / cancel | ✅ (Return in a multi-line field is a new line) |
| Edit popup | middle button | close the comments popup | ↻ comments are edited inline in the popup |
| Unit panel (`w_rulers.c`) | Return / Escape | set / cancel | ✅ both close it (settings apply as they change) |
| Indicator value panels (`w_indpanel.c`) | Return / Escape / Meta-Q | set / cancel / cancel | ✅ values apply as they change, Escape closes (egui popups); Meta-Q not ported (it quits in the global keymap) |
| Indicator buttons | wheel up / down | previous / next value (middle / right button) | ✅ |
| Indicator spinners | wheel | step the value (up decreases) | ✅ |
| Font panel | Escape | cancel | ✅ (egui popup) |
| Message panel | Return, Escape | dismiss | ✅ |
| File panel, directory browser | Return, double-click, right click (parent) | load/save/merge, change directory | ↻ the system's file dialogs |
| Colour panel (`w_color.c`) | Return, double-click, lock toggles | set colour | ❌ E4 (colour editor) |
| Library panel | Return, double-click, Escape | place object, dismiss | ✅ D6 (double-click an object; Escape closes the window; keys r l h v s S and Shift-l in library mode) |
| Keyboard panel (`w_keyboard.c`) | Return, Escape, Up/Down, Ctrl-P/N | enter, ignore, history | ❌ D11 |
| Spell check panel | Escape, u, v, x | close, update, convert, chop | 🟡 Escape closes (F5); the other keys do not apply to RustFig's panel |
| Mode panel | middle/right button on a mode | nothing (`notused_middle`, `notused_right`) | ✅ (nothing either) |
| Depth panel (`w_layers.c`) | left click, left drag | toggle, sweep | ✅ (W3) |
| Depth panel | right click | set the depth of new objects to that layer | ✅ (`indicator.set`) |
| Rulers (`w_rulers.c`) | left, middle drag, right; Meta+right drag; wheel (buttons 4, 5) | pan, drag, pan; drag; pan | ✅ (W6, W11: wheel and left-drag added) |
| Units box | left / right | pan to origin / Unit menu | ✅ (W6) |

## Revision history

| Date | Note |
|---|---|
| 2026-10-22 | D3: Chop mode key `x` (`ModeChopObject`) added. |
| 2026-10-22 | D16: Trim, split and extend mode key `w` (RustFig's own; xfig has none). |
| 2026-10-22 | D4: Tangent mode key `n` (`ModeTangent`) added. |
| 2026-10-15 | Balloons toggle, Alt+wheel mode menus, compose key, Unit panel and spinner keys; Ctrl-U and Ctrl-X decided. |
| 2026-10-15 | Edit popup Return/Ctrl+Return/Escape; depth panel right click. |
| 2026-10-15 | Menus: Meta-F/E/V/H and the letters inside them; plain-letter shortcuts are off while a menu or popup is open. |
| 2026-10-15 | First audit (W8): arrows, Home, Home/End in text, Shift-Meta-X, wheel on indicator buttons, Escape in panels and Return in the search panel added. |
| 2026-10-21 | W16: no modern scheme: Shift+left in Open compound and the right-button split in Join/Split went with it; the left+right chord, Option/Alt+click and macOS Ctrl+click remain. |
| 2026-10-21 | Mouse buttons xfig leaves free now do something (W13, W14): Update right = region; Move right = region move; Delete point middle = region; Add point middle = point at an end; Arrow right = reverse; Break right = all levels; Align right = to first object; Box/ArcBox/Picture middle = from the centre (Shift: square); Picture right = keep aspect; Regular polygon middle = by one side; Circle right = through three points; Text middle (clipboard holds text) = paste. xfig's own meanings are unchanged; the modern scheme has none of them. |
| 2026-10-21 | Region selections (Glue middle, Delete middle, Update right, Move right, Delete point middle): press and drag draws a lasso (W15); click, move, click keeps xfig's rectangle. |
| 2026-10-23 | W21: modifiers as ⌃ ⌥ ⇧ ⌘ (macOS) or Ctrl+Alt+Shift, punctuation keys spelled, both keys per menu entry, Help > Keys for the running system only. |
