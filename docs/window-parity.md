# Window-by-window comparison with xfig (W10)

**Status:** 2026-10-21. Group 1 (main window and menus) compared; groups 2 to 5 not yet compared.

## Rule (decided 2026-10-21)

RustFig may look nicer and more modern than xfig; the **functionality** must stay the same or grow. Windows are compared by what they offer and do (entries, options, ranges, defaults, behaviour, shortcuts, enabled/greyed state, wording where it names a function), not by pixel look, fonts, colours, spacing or widget style. Look differences are recorded only as "look (allowed)"; RustFig is not changed to look like xfig. Every function xfig has that RustFig lacks is a finding (fixed, or a gap with a task id); RustFig extras are listed as extras. Deliberate modernisations (system file dialogs, printing, Modern layout) are marked as such.

## Method and what could be captured

- Reference: xfig 3.2.9a started as `/opt/homebrew/bin/xfig` (standard defaults, no wrapper flags). This installation is **xfig-qt** (xfig 3.2.9a's source on Qt 6, a native macOS window, no X server). XQuartz's `xwd` therefore sees nothing, and `screencapture`, `osascript` and the Accessibility tree gave no window image or menu access from the agent's sandbox ("could not create image from display"). **No screenshot of xfig could be taken and no xfig menu could be opened programmatically**, so group 1 was compared against xfig's sources (`old/xfig-3.2.9a/src/w_cmdpanel.c`, `w_modepanel.c`, `w_indpanel.c`, `w_mousefun.c`, `w_msgpanel.c`, `w_rulers.c`, `w_layers.c`, `w_snap.c`, `main.c`, `app-defaults/Fig`), which are the ground truth for labels, order, accelerators and defaults. A later pass with screen-recording permission should add the xfig images (`target/window-parity/xfig-*.png`).
- RustFig: `target/window-parity/rf-main.png` (Classic layout, 2560x1652, control channel `ui.screenshot`).

## Groups

| Group | Windows | Status |
|---|---|---|
| 1 | Main window and all menus | compared 2026-10-21 (below) |
| 2 | File dialogs: Open, Save As, Merge, Export, Print | not yet compared |
| 3 | Edit panels per object kind: arc, box/arc-box, circle/ellipse, polyline/polygon, spline, text, picture, compound, regular polygon, dimension line | not yet compared |
| 4 | Global settings, colour editor, font panel, style manager, search/replace, spell check, units, libraries, character map, about, set-value/choice popups of the zoom and unit boxes | not yet compared |
| 5 | Attribute pop-ups (line style, width, fill, colour, arrows, joins, caps, justification, ...) and mode panels' option panels | not yet compared |

## Group 1: main window

| Window | xfig | RustFig | Status | Differences |
|---|---|---|---|---|
| Whole window, Classic layout | source only (see above) | `target/window-parity/rf-main.png` | same functions | Arrangement follows xfig (menus, file name and message panel on top, mouse-function box top right, mode panels left, rulers, canvas, depth panel and Snap Mode right, indicator row at the bottom). Look (allowed): fonts, icon rendering at HiDPI, colours. Extra: Modern layout (View > layout). |
| Top strip: menu buttons, current file name, message panel | File, Edit, View, Snap, Help; `READY. Select a mode or load a file` (`main.c`) | same words | same | Look (allowed): file name box. Extra: file name marks unsaved changes; message history by click. |
| Mouse-function panel | title "Mouse Buttons", three rows, per mode and state (`w_mousefun.c`) | same title, per-tool text from `Engine::mouse_hints()` (a test checks it against the tools) | same | Missing mouse functions are listed under the modes (D4, D5, D10 etc.). |
| Mode panel: drawing | 15 buttons in `mode_switches` order (ellipse by radius ... text, library) | same order and icons | same except D6 | Library button greyed with tooltip (D6). Extra: Alt+wheel pop-up of the panels (xfig's `PopupModePanel`) exists. |
| Mode panel: editing | 25 buttons (glue, break, open compound, join/split, chop, scale, align, move point, move, add point, copy, delete point, delete, update, edit, flip UD/LR, rotate CW/CCW, convert, auto arrow, tangent, angle/length/area measure) | same order and icons | same except D1-D5 | Open compound (D1), join/split (D2), chop (D3), tangent/normal (D4), three measure modes (D5) greyed with tooltips. |
| Attribute (indicator) panel | `ind_switches` in `w_indpanel.c`: Zoom, Grid Mode, Point Posn, Depth, Rotn Angle, Num Sides, Num Copies (X, Y), Vert/Horiz Align, Angle Geom, colours, Fill Style, Arc Type, Smart Links, Line Width/Style, Tangent/Norm Length, Join/Cap Style, Arrow Mode/Type/Size, Box Curve, Dimension line, Text Just/Angle/Flags/Size/Step/Font, **Update Control** | xfig's order and per-mode masks, choice panels, middle/right button stepping | partial | Missing: **Update Control** (which attributes Update applies; new W10-1), Angle Geom (D7), Smart Links (D8), Num Copies/X/Y copies (D10), Tangent/Norm Length (D4), Dimension line settings (D10). |
| Rulers | top and left, `w_rulers.c` ticks and labels, cursor marker, pan buttons | same | same | Look (allowed). |
| Units box | `1in = 1.00in` (`1cm`, `1:1.00` for scale), left button pans to origin, right button Set Units | same text, same buttons | same | |
| Depth panel | "Depths" with All On, All Off, Toggle, Gray/Blank, per-depth check boxes (`w_layers.c`) | same labels | same | Extra: object counts per depth, hidden depths cannot be picked. |
| Snap Mode box | "Snap Mode" choice (`w_snap.c`) | "Snap Mode" box with the current mode | same | |
| Canvas | page border, grid, zoom | same | same | Extra: Letter label on page border. |

## Group 1: menus

xfig's menus are in `w_cmdpanel.c`; RustFig's Classic layout builds them from `XFIG_MENUS` (`crates/ui-egui/src/menus.rs`). Accelerators: xfig shows them inside the label ("(Meta-N)"); RustFig shows them separately (look, allowed) and handles the keys in `XFIG_KEYS` (Meta = Alt/Option). Entries RustFig lacks are shown greyed with a tooltip naming the task.

| Menu | xfig entries (order) | RustFig | Differences |
|---|---|---|---|
| File | New, Open, Merge, [Digitize], Save, Save As, Export (quick Shift-Meta-X), Print (quick Shift-Meta-P), Exit; recent files numbered 1-9 | same order and words, recent files with 1-9 | Print greyed (B5, modernised printing: deliberate). Export quick (Shift-Meta-X) works. Digitize only exists when xfig is built with `-DDIGITIZE` (not the standard build): shown greyed, no task. Open/Merge/Save As/Export use system file dialogs (deliberate modernisation, group 2). Extras in the Modern layout: Import picture. |
| Edit | Undo (Meta-U), Paste Objects (Meta-T), Paste Text (F18/F20), Search/Replace (Meta-I), Spell Check (Meta-K), Delete All (Meta-D), divider, Global settings (Meta-G), Set units (Shift-U) | same order and words | Paste Text greyed (E7), Spell Check greyed (F5). Extras: Redo, Cut, Copy, Duplicate, Delete, Select All/None, Edit selected in the Modern layout; RustFig preferences inside Global settings. |
| View | Manage Styles (Ctrl-Y), Redraw (Ctrl-L), Portrait/Landscape (Meta-C), Zoom In (Shift-Z), Zoom Out (z), Zoom to Fit canvas (Ctrl-Z), Unzoom, Pan to origin, Character map, divider, Show page borders (Meta-B), Show depth manager, Show info balloons (Meta-Y), Show line lengths (Meta-L), Show vertex numbers, Autorefresh mode | same order and words, check marks on the toggles | Greyed: Manage Styles (E6), Character map (E5), Show line lengths (D10), Show vertex numbers and Autorefresh mode (no task: see W10-3). Defaults match `app-defaults/Fig` (borders, depth manager and balloons on). Extras in Modern layout: grid, indicator, layout switch, xfig mouse buttons, xfig keys. |
| Snap | Hold, Release, divider, Endpoint, Midpoint, Nearest, Focus, divider, Normal, Tangent, Intersection, Diameter | same words and order, current mode selected | none |
| Help | Xfig Reference (HTML), Reference (PDF), Man Pages (HTML), How-To Guide (PDF), About Xfig | "RustFig Reference (commands)", "About RustFig" | Documentation entries differ (W10-2). Deliberate: RustFig's own About and command reference. |
| Menu letters | letter picks an entry while a menu is open (`Fig*commands*...translations`) | `XFIG_MENU_LETTERS`, same letters | Letters of greyed entries do nothing. |
| Object, Tools (Modern layout) | none in xfig | Group/Ungroup, Update, Convert, Arrows, Transform, Arrange, Measure | Extras (functions exist as modes or keys in Classic). |

## Findings and ids

| Id | Difference | Fixed or task |
|---|---|---|
| W10-1 | Update Control: xfig's Update mode applies only the attributes ticked in the indicator panel's "Update Control" buttons (`upd_ctrl` in `w_indpanel.c`); RustFig's Update copies all | plan W10-1, gaps #29 |
| W10-2 | Help menu has no link to documentation (xfig: reference, man pages, how-to) | plan W10-2, gaps #29 |
| W10-3 | View > Show vertex numbers and Autorefresh mode missing (vertex numbers are a debugging aid; autorefresh is moot with a retained canvas) | plan W10-3, gaps #29 |
| B5, E5, E6, E7, F5, D1-D8, D10 | greyed menu entries and mode buttons, already planned | unchanged |

Nothing in group 1 needed an in-place fix: labels, order, accelerators and defaults of the five menus already equal `w_cmdpanel.c` and `app-defaults/Fig`.

## Revision history

| Date | Note |
|---|---|
| 2026-10-21 | Created; group 1 compared from xfig's sources (no xfig capture possible, see Method). |
