# xfig key and mouse bindings checklist (W8)

**Status:** 2026-10-15. First audit. Every binding xfig defines, from
`app-defaults/Fig` and the 78 translation tables (210 bindings) in xfig's
`src/*.c`, with what RustFig does. X plumbing (window-manager close
messages, expose and resize events, highlight on enter/leave) is left out:
egui does those itself.

Legend: ✅ works as in xfig · 🟡 partly · ❌ missing (task) · ↻ replaced by a
modern equivalent. Meta is Alt (Option on macOS); xfig binds Meta and Alt
alike. RustFig's own platform shortcuts (Ctrl/Cmd+S, …) work alongside.

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
| Shift-Meta-P | Print with current settings | — | ❌ B5 (printing) |
| Meta-P | Print panel | — | ❌ B5 |
| Meta-Q | Quit | `app.quit` | ✅ |
| Shift-U | Unit panel | `app.unit_panel` | ✅ |
| Meta-U | Undo | `edit.undo` | ✅ |
| Meta-T | Paste | `edit.paste` | ✅ |
| Meta-I | Search | `app.search` | ✅ |
| Meta-K | Spell check | — | ❌ D11 (spell check) |
| Meta-G | Global settings | `app.global_settings` | ✅ |
| Meta-D | Delete all | `edit.delete_all` | ✅ |
| Ctrl-L, Meta-R | Redraw | `app.redraw` | ✅ |
| Meta-C | Change orientation | `doc.toggle_orientation` | ✅ |
| Shift-Z / Z / Ctrl-Z | Zoom in / out / fit | `view.zoom_in`, `view.zoom_out`, `view.zoom_fit` | ✅ |
| Ctrl-Y | Named styles | — | ❌ E6 (style manager) |
| Meta-B | Show page borders | `view.toggle_page_border` | ✅ |
| Meta-Y | Show balloons (tooltips) | — | ❌ W8 |
| Meta-L | Show line lengths while drawing | — | ❌ W8 (needs the length display) |
| Meta-F/E/V/H | Pop up the File/Edit/View/Help menu | `menus::XFIG_MENU_KEYS` | ✅ |

## Letters while a menu is open (`Fig*commands*…*translations`)

| Menu | Letters | Status |
|---|---|---|
| File | n o m d s a e p x; 1–9 recent files | ✅ letters (`XFIG_MENU_LETTERS`; d, p wait for D11, B5); ❌ recent files E1 |
| Edit | u p t f s d g n | ✅ (t waits for E7, s for D11) |
| View | s r t i o f u p c d n b l a, v v | ✅ for the entries RustFig has; named styles (E6), character map (E5), balloons, lengths, auto refresh, vertex numbers (W8) wait for their features |
| Help | x m h a | ✅ x (command reference), a (about); m, h: xfig's manuals (D11) |

## Canvas (`w_canvas.c`)

| Binding | xfig action | Status |
|---|---|---|
| Left/Right/Up/Down | pan by a ruler step (Shift: five) | ✅ |
| Home | pan to the origin | ✅ (`view.pan_origin`) |
| Left/Right/Home/End while typing | move the caret | ✅ |
| Wheel / Shift+wheel | pan vertically / horizontally | ✅ |
| Ctrl+wheel | zoom | ✅ |
| Ctrl+left/middle/right | zoom area / pan to origin / unzoom | ❌ E3 (zoom box) |
| Alt+wheel up/down | pop up the drawing/editing mode menus | ❌ W8 |
| Ctrl-K | keyboard coordinate entry panel (`w_keyboard.c`) | ❌ D11 |
| Meta (Compose) + two keys | compose characters (`CompKeyDB`) | ❌ W8 |
| F18, F20 (Sun Paste keys) | paste | ↻ Ctrl/Cmd+V |

## Panels and fields

| Where | Binding | xfig action | Status |
|---|---|---|---|
| Text fields (`*Text*translations`) | Delete, Home, End | delete forward, line start, line end | ✅ (egui) |
| Numeric fields (`resources.c`, `w_indpanel.c`, export, print) | Ctrl-U | multiply the next step by 4 | ❌ W8 |
| Text fields | Ctrl-X | clear the field | ❌ W8 |
| Text fields | Ctrl-J, Ctrl-M, Return | no new line | ↻ single-line fields |
| Search panel | Return in Find / Replace | search / replace | ✅ |
| Search panel | Escape, Meta-C, Meta-Q | close | ✅ Escape; ❌ Meta-C/Q (W8) |
| Export, Global, Document, Commands, About panels | Escape | cancel | ✅ |
| Export panel | Return | export; in Magnification: apply | ❌ W8 |
| Edit popup (`e_edit.c`) | Return / Ctrl+Return / Escape | apply / done / cancel | ❌ W8 |
| Edit popup | middle button | close the comments popup | ❌ W8 |
| Unit panel (`w_rulers.c`) | Return / Escape | set / cancel | ❌ W8 |
| Indicator value panels (`w_indpanel.c`) | Return / Escape / Meta-Q | set / cancel / cancel | ❌ W8 |
| Indicator buttons | wheel up / down | previous / next value (middle / right button) | ✅ |
| Indicator spinners | wheel | step the value | ❌ W8 |
| Font panel | Escape | cancel | ❌ W8 |
| Message panel | Return, Escape | dismiss | ❌ W8 |
| File panel, directory browser | Return, double-click, right click (parent) | load/save/merge, change directory | ↻ the system's file dialogs |
| Colour panel (`w_color.c`) | Return, double-click, lock toggles | set colour | ❌ E4 (colour editor) |
| Library panel | Return, double-click, Escape | place object, dismiss | ❌ D6 (libraries) |
| Keyboard panel (`w_keyboard.c`) | Return, Escape, Up/Down, Ctrl-P/N | enter, ignore, history | ❌ D11 |
| Spell check panel | Escape, u, v, x | close, update, convert, chop | ❌ D11 |
| Mode panel | middle/right button on a mode | (xfig's PressMiddle/PressRight) | review in W8 |
| Depth panel (`w_layers.c`) | left click, left drag | toggle, sweep | ✅ (W3) |
| Depth panel | right click | set the current depth to that layer | ❌ W8 |
| Rulers (`w_rulers.c`) | left, middle drag, right; Meta+right drag | pan, drag, pan; drag | ✅ (W6) |
| Units box | left / right | pan to origin / Unit menu | ✅ (W6) |

## Revision history

| Date | Note |
|---|---|
| 2026-10-15 | Menus: Meta-F/E/V/H and the letters inside them; plain-letter shortcuts are off while a menu or popup is open. |
| 2026-10-15 | First audit (W8): arrows, Home, Home/End in text, Shift-Meta-X, wheel on indicator buttons, Escape in panels and Return in the search panel added. |
