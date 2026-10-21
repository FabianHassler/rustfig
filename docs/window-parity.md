# Window-by-window comparison with xfig (W10)

**Status:** 2026-10-21. Group 1 (main window and menus) compared and re-checked against screenshots of the real xfig; group 2 (file windows: Open, Save As, Merge, Export, Print) compared; group 3 (object edit panels) compared with captures and fixed in part; groups 4 and 5 not yet compared.

## Rule (decided 2026-10-21)

RustFig may look nicer and more modern than xfig; the **functionality** must stay the same or grow. Windows are compared by what they offer and do (entries, options, ranges, defaults, behaviour, shortcuts, enabled/greyed state, wording where it names a function), not by pixel look, fonts, colours, spacing or widget style. Look differences are recorded only as "look (allowed)"; RustFig is not changed to look like xfig. Every function xfig has that RustFig lacks is a finding (fixed, or a gap with a task id); RustFig extras are listed as extras. Deliberate modernisations (system file dialogs, printing, Modern layout) are marked as such.

## Method and what could be captured

- Reference: the real xfig 3.2.9a (X11, Xaw3d), `/opt/homebrew/bin/xfig`, run directly with its standard defaults on a hidden virtual display (`Xvfb :77 -screen 0 1600x1200x24`, so nothing appears on the user's screen and the user's X resources are not loaded), driven with a small XTest program (press, move, release; XQuartz has no XTEST, Xvfb has) and captured with `xwd -root | xwdtopnm | pnmtopng`. Images: `target/window-parity/xfig-*.png` (not committed). Two things the user's home leaks into xfig: the File menu's recent-file list (read from `~/.xfigrc`) and nothing else. The earlier note that this xfig was a Qt build and could not be captured was wrong (it was an older install); the sources remain the ground truth for labels and behaviour.
- RustFig: `target/window-parity/rf-*.png` (control channel `ui.screenshot`, Classic layout, 2560x1652) and, since A11 (`ui.click_label` opens menus and pop-ups), `a11-menu-{file,edit,view,snap,help}.png`, `a11-popup-linestyle.png`, `a11-submenu-tools-draw.png` (Modern layout), `a11-dialog-export.png`; the dialogs that open by command (`app.export`) are imaged.
- xfig's manual images (`doc/html/images/`) copied as `xfig-manual-*.png` for comparison with the captures: they match the 3.2.9a panels except for details noted below.

## Groups

| Group | Windows | Status |
|---|---|---|
| 1 | Main window and all menus | compared 2026-10-21 (below) |
| 2 | File dialogs: Open, Save As, Merge, Export, Print | compared 2026-10-21 (below) |
| 3 | Edit panels per object kind: arc, box/arc-box, circle/ellipse, polyline/polygon, spline, text, picture, compound, regular polygon, dimension line | compared 2026-10-21 (below); position rows, text flags, arrow rows, arc type, figure comments, spline point panel and Cancel-restores fixed; picture and compound rows open (W10-15, W10-16) |
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
| Object markers (W12) | `u_markers.c`: 5 px squares on the objects each editing mode acts on (mask table in `engine/tools/markers.rs`) | same objects and points; hollow squares in the theme's handle colour | Look (allowed). Screenshots `target/window-parity/w12-*.png`. |
| Rulers | top and left, `w_rulers.c` ticks and labels, cursor marker, pan buttons | same | same | Look (allowed). |
| Units box | `1in = 1.00in` (`1cm`, `1:1.00` for scale), left button pans to origin, right button Set Units | same text, same buttons | same | |
| Depth panel | "Depths" with All On, All Off, Toggle, Gray/Blank, per-depth check boxes (`w_layers.c`) | same labels | same | Extra: object counts per depth, hidden depths cannot be picked. |
| Snap Mode box | "Snap Mode" choice (`w_snap.c`) | "Snap Mode" box with the current mode | same | |
| Canvas | page border, grid, zoom | same | same | Extra: Letter label on page border. |

## Group 1: menus

Captured from the real xfig: `xfig-main.png` (main window), `xfig-menu-file.png`, `xfig-menu-edit.png`, `xfig-menu-view.png`, `xfig-menu-snap.png`, `xfig-menu-help.png`, `xfig-popup-gridmode.png` (the Grid Mode pop-up panel: NONE and eight grid pictures, Cancel; its function is group 5). Every label, order, divider and accelerator shown in the captures equals the tables below (the Edit menu shows `Paste Text (F18/F20)`, Snap has the two dividers, View has the check marks on page borders, depth manager and info balloons). xfig's five menus are flat lists: **there are no cascades or sub-menus** in File, Edit, View, Snap or Help; the only drop-downs from the main window are the Grid Mode pop-up panel (indicator button), the other attribute pop-ups (group 5) and the unit box's Set Units panel (group 4). Entries of xfig's File menu in the capture: New (Meta-N), Open... (Meta-O), Merge... (Meta-M), Save (Meta-S), Save As... (Meta-A), Export... (Meta-X, Quick = Shift-Meta-X), Print... (Meta-P, Quick = Shift-Meta-P), Exit (Meta-Q), then the numbered recent files (5 in the capture).

xfig's menus are in `w_cmdpanel.c`; RustFig's Classic layout builds them from `XFIG_MENUS` (`crates/ui-egui/src/menus.rs`). Accelerators: xfig shows them inside the label ("(Meta-N)"); RustFig shows them separately (look, allowed) and handles the keys in `XFIG_KEYS` (Meta = Alt/Option). Entries RustFig lacks are shown greyed with a tooltip naming the task.

Compared again with the A11 screenshots `a11-menu-*.png` against `xfig-menu-*.png` (2026-10-21): entries, order, dividers, check marks and greyed items equal the tables. Two differences found and fixed: Unzoom showed RustFig's own shortcut (xfig: none) and Zoom Out showed "Z" (xfig: "(z)"); in the Classic layout an entry now shows xfig's accelerator or none. Look only: RustFig prints accelerators in a right column without parentheses, menus are rounded and fade in.

| Menu | xfig entries (order) | RustFig | Differences |
|---|---|---|---|
| File | New, Open, Merge, [Digitize], Save, Save As, Export (quick Shift-Meta-X), Print (quick Shift-Meta-P), Exit; recent files numbered 1-9 | same order and words, recent files with 1-9 | Print greyed (B5, modernised printing: deliberate). Export quick (Shift-Meta-X) works. Digitize only exists when xfig is built with `-DDIGITIZE` (not the standard build): shown greyed, no task. Open/Merge/Save As/Export use system file dialogs (deliberate modernisation, group 2). Extras in the Modern layout: Import picture. |
| Edit | Undo (Meta-U), Paste Objects (Meta-T), Paste Text (F18/F20), Search/Replace (Meta-I), Spell Check (Meta-K), Delete All (Meta-D), divider, Global settings (Meta-G), Set units (Shift-U) | same order and words | Paste Text greyed (E7), Spell Check greyed (F5). Extras: Redo, Cut, Copy, Duplicate, Delete, Select All/None, Edit selected in the Modern layout; RustFig preferences inside Global settings. |
| View | Manage Styles (Ctrl-Y), Redraw (Ctrl-L), Portrait/Landscape (Meta-C), Zoom In (Shift-Z), Zoom Out (z), Zoom to Fit canvas (Ctrl-Z), Unzoom, Pan to origin, Character map, divider, Show page borders (Meta-B), Show depth manager, Show info balloons (Meta-Y), Show line lengths (Meta-L), Show vertex numbers, Autorefresh mode | same order and words, check marks on the toggles | Greyed: Manage Styles (E6), Character map (E5), Show line lengths (D10), Show vertex numbers and Autorefresh mode (no task: see W10-3). Defaults match `app-defaults/Fig` (borders, depth manager and balloons on). Extras in Modern layout: grid, indicator, layout switch, xfig mouse buttons, xfig keys. |
| Snap | Hold, Release, divider, Endpoint, Midpoint, Nearest, Focus, divider, Normal, Tangent, Intersection, Diameter | same words and order, current mode selected | none |
| Help | Xfig Reference (HTML), Reference (PDF), Man Pages (HTML), How-To Guide (PDF), About Xfig | "RustFig Reference (commands)", "About RustFig" | Documentation entries differ (W10-2). Deliberate: RustFig's own About and command reference. |
| Menu letters | letter picks an entry while a menu is open (`Fig*commands*...translations`) | `XFIG_MENU_LETTERS`, same letters | Letters of greyed entries do nothing. |
| Object, Tools (Modern layout) | none in xfig | Group/Ungroup, Update, Convert, Arrows, Transform, Arrange, Measure | Extras (functions exist as modes or keys in Classic). |

## Group 2: file windows

Sources: `w_file.c` (Open, Save As, Merge share one panel), `w_dir.c` (the file list part), `w_export.c`, `w_print.c`, `u_print.c` (what is passed to fig2dev and lp), `f_util.c` (`ok_to_write`, `emptyfigure_msg`), `f_load.c`, `f_save.c`. Captures: `xfig-open.png`, `xfig-open-preview.png`, `xfig-saveas.png`, `xfig-merge.png`, `xfig-export-eps.png`, `xfig-export-langmenu.png`, `xfig-export-ps-family.png` (PostScript, EPS, PDF, EPS and PDF), `xfig-export-latex-family.png`, `xfig-export-other-family.png`, `xfig-export-bitmaps.png`, `xfig-export-ps.png`, `xfig-export-pdf.png`, `xfig-export-gif.png`, `xfig-export-jpeg.png`, `xfig-export-ibmgl.png`, `xfig-export-menu-papersize.png`, `xfig-export-menu-pages.png`, `xfig-export-menu-offunit.png`, `xfig-export-menu-background.png`, `xfig-print.png`; manual images `xfig-manual-file-panel.png`, `xfig-manual-export-panel*.png`, `xfig-manual-print-panel.png`. RustFig: `rf-export.png`.

**Deliberate modernisation (decided 2026-10-14, plan B5 and the file-dialog decision):** xfig's own directory browser (Filename Mask, Current Dir, Directories, Home, Show Hidden, Rescan, "Fig files"/"Existing" lists, the file name text field, "Default File") is replaced by the operating system's file dialogs. The functions that live in that browser and that nothing else in RustFig replaces are findings below (offset, preview, comments, status, new xfig).

### Open, Save As, Merge (one panel in xfig)

| Function | xfig | RustFig | Status |
|---|---|---|---|
| Panel title and buttons | one panel; buttons Rescan, Cancel, Open / Save / Merge, New xfig... | system dialog (Open, Save, Merge), Cancel | modernised |
| Modified figure before Open | query "The current figure is modified. Do you want to discard it and load the new file?" (Yes / Cancel) | query "Save changes? Save, Discard, Cancel" (also for New and Exit) | same or more (RustFig offers Save) |
| Open a name that does not exist | the name becomes the current file, empty figure, "(new file)" | error "No such file" | difference (W10-8) |
| Open messages | "File "x" does not exist / is a directory / is empty / has no version number in header", "Read access ... blocked" | error text from the OS or the reader | same function, wording differs |
| Figure Offset X / Y (units: Inches, Centim., Fig Units) on Open and Merge | moves the loaded figure | engine: `file.open` and `file.merge` take `offset_x`, `offset_y` (Fig units), done in W10 (tests `file_windows.rs`); no field in the UI, because the system dialog has none | engine done, UI W10-6 |
| Merge result | the merged objects become one compound (or the file's single compound as it is), undoable as one step, offset applied | now the same (was loose objects); merged compound is selected | fixed in W10 |
| Merge: colours and picture colours | user colours mapped into the figure | user colours mapped | same |
| Preview of the selected .fig (picture, size "4.9 x 3.9 in", name, Stop Preview) | in the panel before opening | none | gap W10-6 |
| "Figure comments:" of the selected file | shown | none | gap W10-6 |
| File Status ("Modified" / "Not modified") and "# of Objects" | shown | the title shows the modified mark; no object count | minor (W10-6) |
| Current File | shown | window title | same function |
| Filename: no suffix gets ".fig" | yes (Open, Merge, Save) | the system dialog adds the suffix on save; .rustfig also offered | same or more |
| Save As: file exists and name changed | query ""x" already exists. Do you want to overwrite it?" (Yes / No) | the system save dialog asks | modernised |
| Save: empty figure | message "Empty figure, Save command ignored" | saves an empty figure | extra (more permissive; kept) |
| Save: unsaved name | opens Save As ("No filename, please enter name") | opens Save As | same |
| Save with open compound (inside a compound) | query "save just the visible part or all" | not applicable until open compound exists (D1) | follows D1 |
| Backup: `write_bak` (default true) renames the old file to `name.bak` before saving | yes | atomic write through a temporary file, no `.bak` | difference (W10-9: setting) |
| Read-only / directory / permission messages (`ok_to_write`) | "is a directory", "Write permission denied", "is read only" | the OS error is shown | same function |
| New xfig... button | starts another xfig on the chosen file | none (RustFig has one window) | no equivalent (record, low) |
| Recent files | File menu, 1 to 9 | same | same |
| Directory of the figure for relative picture names | `cur_file_dir` | directory of the file | same |
| Esc / Return in the panel | Esc cancels, Return does the action | handled by the system dialog | modernised |

### Export panel

xfig's panel is "Xfig: Export menu". Rows are shown only for the languages that use them (`manage_optional`). The language list is identical to RustFig's: 39 entries in the same order (PostScript ... XPM) with a "Bitmap Formats" divider before GIF; RustFig's combo box lists the same descriptions without the divider (look). Defaults: xfig starts with EPS (resource `exportLanguage`), RustFig with PDF (kept; user's choice earlier: PDF is the modern default, W10-4 notes it).

| Row | xfig (languages that show it) | RustFig | Status |
|---|---|---|---|
| Language | menu of 39 | combo of the same 39, languages that are not ported are greyed with the task id | same (B9 languages pending: see plan) |
| Magnification % (spinner), "Fit to Page", "Figure size: 4.9in x 3.9in" (live) | all | magnification only; no Fit to Page, no live size | gap W10-4 |
| Export all layers / Export only active layers / Boundary only active layers | all | "active layers only" with "bounds: whole figure / active layers" (enabled when a depth is off) | same function |
| Border Margin (points) | PS, EPS, PDF, PSTeX, PS/PDF, pict2e, TikZ, HTML map, all bitmaps | always shown, in Fig units (1/1200 in), converted to points; shown for languages that ignore it | difference (units and which languages; W10-4) |
| Background colour (menu: None, Default, every colour) | PS, EPS, PDF, PSTeX, PS/PDF, Tk, bitmaps | checkbox "white (otherwise transparent)" for every language | gap W10-4 (any colour, None) |
| Grid Minor / Major (menus and text, in the current unit) | PS, EPS, PDF, PSTeX, PS/PDF, bitmaps | none (engine option `-G` exists) | gap W10-4 |
| Add Preview (None, ASCII (EPSI), Color TIFF, Monochrome TIFF) | PS, EPS | none (fig2dev options `-A`/`-C`/`-T` exist, B4b) | gap W10-4 |
| PDF: "Page mode" check box and "Crop to Bounding Box" (on = cropped) | PDF | none; PDF is cropped | gap W10-4 |
| Paper Size (29 sizes: Letter, Legal, Tabloid, ANSI A-E, ISO A9-A0, JIS B10-B0) and Fit to Page | PS, PDF in page mode, HPGL, (DXF has orientation only) | none in the dialog; paper (15 Fig-file sizes) and orientation are document settings (Global settings, View > Portrait/Landscape) | gap W10-4 (RustFig's model has the 15 sizes of the Fig header; xfig's table has 29) |
| Orientation, Justification (Centered / Flush left), Pages (Single / Multiple), Overlap | PS, PDF page mode (orientation also HPGL, DXF); justification greyed when Multiple, overlap enabled only when Multiple | document settings hold orientation, justification and multiple pages; the dialog has none of them, and no overlap | gap W10-4 |
| Offset X / Y (Inches, Centim., Fig Units) | PS, PDF page mode, HPGL | none | gap W10-4 (note: xfig's `u_print.c` passes `-y` with the x value, a bug fig2dev's `-y` makes visible; RustFig should do it right) |
| HPGL: "Issue PCL Command to use HPGL", "Use Specified Font" | HPGL/2 | none (language not ported yet, B9) | follows B9 |
| Bitmap Options: Smoothing (No / Some / More) | all bitmaps | Smoothing for bitmaps; default More (xfig: none), deliberate | same function |
| GIF Transparent colour (menu: None, Background, colours) | GIF | none in the dialog (document setting `transparent_color` exists) | gap W10-4 |
| JPEG Image quality (%) 1-100, default 75 | JPEG | Image quality 1-100, default 75 | same |
| (RustFig only) Resolution in dpi | none (xfig scales bitmaps by the magnification) | bitmaps | extra |
| Default File / Output File / Existing / Filename Mask / Current Dir / Directories | name = figure name + language suffix (`.tif`, `.jpg`, `.pdf` for pdftex, `.eps` for PS/PDF-pair) | system save dialog proposing the same name and suffix | modernised |
| Export button, Cancel, Esc (Cancel), Return in the name field (Export) | yes | Export... , close, Esc, Return | same |
| Quick export (Shift-Meta-X) | exports to the default file with the panel's settings, no panel | same (`app.export_now`) | same |
| Modified figure | query "The current figure is modified. Do you want to save it before exporting?" (xfig exports from the saved file) | exports the figure in memory, no query | deliberate difference: nothing to save first |
| Empty figure | "Empty figure, Export command ignored" | Export command disabled | same function |
| Output file is one of the figure's picture files | "x is an input file. Export aborted" | now refused with the same message (engine, W10) | fixed in W10 |
| Overwrite | query "x already exists. Do you want to overwrite it?" unless it is the default name | the system dialog asks | modernised |
| Messages | `Exporting to file "x" in PORTRAIT mode ...`, `Export to "x" done` / `failed` | "Exported x" | same function |
| Combined languages (PS/LaTeX, PDF/LaTeX, PS/PDF/LaTeX, EPS and PDF) write two or three files | yes (`x.pstex`, `x.pstex_t`) | yes (`file.export` writes the parts) | same |
| File mask after choosing a language (`*.eps`, quirk `*.pspdf`) | yes | system dialog filters by the suffix | modernised |

### Print panel

xfig: "Xfig: Print menu", "Print to PostScript Printer". RustFig: **File > Print is greyed** (B5, modernised printing; no panel).

| xfig element | What it does | RustFig |
|---|---|---|
| Magnification %, Fit to Page, Fig Size | scale to the page | none |
| Paper Size (29), Orientation, Justification, Pages Single/Multiple, Overlap | same options as Export (shared state) | none |
| Background colour, Grid minor/major | like Export | none |
| Print Command (lp, lpr), Printer (text and a menu from /etc/printcap, default `$PRINTER`), Print Job Params (`%f` becomes the file name) | the spool command line | modernised: the OS print dialog (see proposals) |
| Figures in batch, Print FIGURE to Batch, Clear Batch, Print BATCH to Printer | collect several figures in one spool file, print them together | none |
| Print all layers / Print only active / Boundary only active layers | layer choice | the Export option exists; Print not built |
| Print FIGURE to Printer, Dismiss; accelerators Meta-P (print), Meta-B (batch), Esc | | none |
| File > Print quick (Shift-Meta-P) | prints without the panel | greyed |

All of it is plan B5 (12 h) plus the findings W10-4 for the options both panels share.

### Findings of group 2

| Id | Difference | Fixed or task |
|---|---|---|
| W10-4 | Export panel options missing: Fit to Page and live figure size; border in points and only for the languages that use it; background colour menu (None, Default, any colour) for the languages that use it; grid minor/major; EPS/PS preview (ASCII, colour TIFF, mono TIFF); PDF page mode / crop; paper size (xfig's 29 sizes), orientation, justification, single/multiple pages with overlap, offset X/Y with units for PS and PDF page mode; GIF transparent colour; HPGL options (with B9); per-language visibility as `manage_optional`; the same shared state in Print | plan W10-4, gaps #30 |
| W10-5 | Print panel absent: all rows above, batch printing | plan B5 (modernised, ask first) |
| W10-6 | Open and Merge: Figure Offset field, preview with size and comments, file status and object count (engine offset done) | plan W10-6, gaps #30 |
| W10-7 | (closed in W10) Merge made loose objects instead of one compound and had no offset; Export did not refuse an input picture file | fixed, tests `crates/engine/tests/file_windows.rs` |
| W10-8 | Open of a name that does not exist should start a new figure with that name (xfig: "(new file)") | plan W10-8, gaps #30 |
| W10-9 | `write_bak`: xfig renames the old file to `.bak` when saving (default on); RustFig writes atomically without a backup. Needs a preference (decide with the user: a `.bak` per save is what xfig users expect) | plan W10-9, gaps #30 |
| W10-10 (note only; the plan's W10-10 is the export preview) | New xfig... button (start another window on the file): no equivalent; low value (record only) | gaps #30 |

### Proposed modernisations (group 2)

Not implemented; the user asked to be asked first. Each keeps every xfig function listed in the tables above.

1. **Print through the operating system's print dialog with a preview (replaces the lp/lpr panel; plan B5).** xfig: Print panel with Print Command lp/lpr, Printer name and printcap menu, Job Params, magnification, paper, orientation, justification, pages, overlap, background, grid, layers, batch. Proposal: File > Print opens one RustFig print dialog with a live page preview (the figure on the sheet at the chosen magnification, with the page border and, for Multiple pages, the tile boundaries), then hands a PDF to the OS print dialog (macOS, Windows, CUPS on Linux; browser print on the web) for printer, copies, page ranges, duplex, colour. Page setup (paper from the OS list plus xfig's 29 sizes, orientation, margins) in the same dialog, remembered per figure through the Fig header (paper, orientation, justification, multiple) and per user for the rest. Keeps: magnification and Fit to Page, paper size, orientation, justification, single/multiple pages and overlap, background, grid, layer choices, quick print (Shift-Meta-P, printing with the last settings), batch (several figures in one job: a "Add to batch" button and a batch list, 4 h of the total) and Job Params for CUPS (`lp` options string) in an Advanced field. Hours: 24 (replaces B5's 12 and adds preview 6, batch 4, page setup 2).
2. **Export dialog with a live preview.** xfig: modal panel of rows, no picture, "Figure size" text. Proposal: the same rows (all of W10-4) with a preview of the result next to them (the page layout for PS/PDF, the bitmap at its size, the bounding box and margin for the others), updated as options change, and the figure size shown in the current units. Keeps every row and the Return / Esc / quick-export behaviour. Hours: 10 on top of W10-4 (14).
3. **Remembered per-format options.** xfig keeps one set of options for the session. Proposal: remember the options per language across sessions (resolution, smoothing, JPEG quality, border, grid, background, preview, paper) in the preferences, with "Reset to xfig's defaults". Hours: 3.
4. **Open, Merge and Save As in one in-app file panel on top of the system dialog's features.** xfig's panel shows the preview, size, comments, object count and Figure Offset. Proposal: a side panel next to the system file dialog is not possible, so offer an optional in-app "Open with preview" (file list from the system dialog's folder, preview, comments, status, Figure Offset X/Y with units, Merge as one compound), still reachable by the OS dialog. Keeps: preview, comments, status, offset and units. Hours: 12 (W10-6 is the plain offset field and preview, 8 of these).
5. **Non-blocking export with progress and "Show in folder".** xfig blocks with a wait cursor and a message line. Proposal: run large exports off the UI thread with a cancel button and a link to the result. Keeps the messages. Hours: 4.
6. **Save safety: autosave versions instead of `.bak`.** xfig writes `name.bak`. Proposal: keep `.bak` as an option (W10-9) and also keep the last N saved versions in the recovery folder. Hours: 4.

Total if all accepted: about 57 h (item 1 24, item 2 10, item 3 3, item 4 12, item 5 4, item 6 4) besides W10-4 (14), W10-6 (8), W10-8 (1), W10-9 (2).

## Group 3: object edit panels

Sources: `e_edit.c` (every panel: `make_window_line/text/ellipse/arc/spline/compound/figure`, `generic_window`, `get_new_*_values`, `done_*`, spline point panel), `w_indpanel.c` (the pop-ups the panel shares with the indicator). Captures of the real X11 xfig (Edit mode, objects of every kind in one figure): `g3-xfig-edit-{box,arcbox,polygon,polyline,picture,ellipse-radii,ellipse-diam,circle-radii,circle-diam,arc-open,arc-pie,spline-open-approx,spline-closed-approx,spline-open-interp,spline-closed-interp,xspline-open,xspline-closed,text,compound,figure-comments,spline-point}.png`, pop-ups `g3-xfig-pop-{pencolor,fillstyle,linestyle,cap,join,arrowtype,units}.png`; RustFig `g3-rf-edit-*.png` (control channel, tool events; the egui combo pop-ups cannot be opened through it, so those are compared from the code). All under `target/window-parity/`, not committed. A regular polygon is a plain polygon in xfig (same panel); a dimension line is a compound (compound panel).

### How the panel works (all kinds)

| Function | xfig | RustFig | Status |
|---|---|---|---|
| Open | Edit mode, left button on an object; the panel opens beside the object. Middle button: **whole-figure comments** panel (Done, Cancel); Shift+middle on an object: shows its comments; right button on a spline control point: **point behaviour panel** | left button as before; middle and right buttons were unused | fixed in W10: middle opens "Whole Figure" comments (command `doc.set_comments` existed, no UI), Shift+middle shows the comments in the message line, right button opens the spline point panel (messages "Cannot edit end-points" and "Only spline points can be edited" as xfig); mouse hints "edit object / edit Main comment / edit point" |
| Done, Apply, Cancel | Done applies and closes (one undo step); Apply applies and stays open; **Cancel restores the object as it was, also after Apply**; Esc cancels, Return applies, Ctrl+Return done (translations) | same keys; Apply used to be final | fixed in W10: Cancel, Esc and the close box undo the Applies; Done and Return as before. Difference kept: each Apply is its own undo step (xfig: Done makes one), W10-17 |
| Live update | none: nothing changes until Apply or Done (only the point panel draws live) | same | same |
| Edit several objects at once | none (one object per panel; Update mode copies attributes) | the panel opens for one object | same (multi-edit is a proposal below) |
| Units of coordinates | "Units:" menu per panel: figure units (in or cm) or "Fig units" | "Units" combo with the same two choices | fixed in W10 (new) |
| Comments | text box in every panel (also compound, figure) | multi-line field in every panel except the figure's | same; the figure's comment panel is new |
| Colour, line style, cap, join, arrow type pop-ups | pictorial pop-up panels (colour grid with Default + 32 + user colours and Cancel; 6 line styles; 3 caps; 3 joins; 29 arrow types as pictures) | combos with the same entries as text (colours with swatches; arrows as "type N hollow/filled") | same function; look (allowed); pictorial pop-ups are a proposal below |
| Fill | "Fill style" menu (No fill, Filled, Pattern) with intensity % field (0 to 200 in steps of 5; tints above 100 not for black/white) or pattern number (0 to 21) and a picture | one combo of None, 0 to 40 (shade/tint, 5% steps) and the 22 named patterns | same function (colour change by the E4 picker is accepted) |
| Ranges | width 0 to 1000 (check_thick clamps to 500), depth 0 to 999, text size 1 to 500, text and ellipse angle -360 to 360, arc-box radius 2 to 1000, arrow thick/width/length 0 to 10000 | width was 0 to 100, radius 0 to 500 | fixed in W10 (width 0 to 1000, radius 0 to 1000; radius 0 stays allowed) |

### Per object kind

| Kind (xfig title) | xfig rows | RustFig now | Status |
|---|---|---|---|
| POLYLINE: Polyline | comments; width, depth; pen, fill colour; fill style, intensity, pattern; line style with dash length; cap style, join style; forward and backward arrow (type, thick, width, length); points N with units and X/Y per point | all rows | fixed in W10 (position rows, arrow type and size per arrow, cap/join only where xfig has them) |
| POLYLINE: Box | same without arrows and cap; join style; first and opposite corner | all rows | fixed in W10 |
| POLYLINE: ArcBox | box rows without join style; corner radius | all rows (radius range widened) | fixed in W10 |
| POLYLINE: Polygon | like polyline without arrows and cap; points | all rows | fixed in W10 |
| (regular polygon) | polygon | polygon | same |
| POLYLINE: Picture Object | pen colour (greyed unless XBM), depth, **file name (editable), Reread, Browse**, type (10 boxes), size in pixels, colours, transparent colour, first/opposite corner, width and length (Return resizes), **orientation Normal / Flipped about diagonal, rotation 0/90/180/270**, current and original h/w ratio, Shrink / Expand to orig, Use original size, Scale by %, buttons Screen Capture and Edit Image | depth, comments, file name (read only), flipped check box, corner rows | partial: flip and corners fixed in W10; rest W10-16 |
| ELLIPSE: by radii / diameters, CIRCLE: by radius / diameter | generic rows; ellipse only: angle; centre X/Y; radii X/Y (circle: one radius) | all rows; start/end points recomputed as xfig does (`get_new_ellipse_values`) | fixed in W10 |
| ARC: Specified by 3 points | arc type menu (Open, Pie Wedge); generic rows; cap; arrows (hidden for pie wedge); first, second, third point; "Invalid ARC points!" when collinear | all rows; collinear points are refused with a message | fixed in W10 |
| SPLINE: open/closed approximated, open/closed interpolated, X-Spline open/closed | generic rows; cap (open only); arrows (not for closed approximated and interpolated); points X/Y (no shape factors in this panel) | all rows; shape factors only through the point panel, as xfig | fixed in W10 |
| Spline point panel (right button) | slider for the shape factor, buttons Approximated, Angular, Interpolated, Done, Cancel; a factor that differs from the kind's turns it into an X-spline; end points refused | same controls; applied on Done (no live drawing: W10-17); Angular = 0, Interpolated = -1 (xfig stores 1e-5 and -0.5) | fixed in W10 |
| TEXT | size, pen colour, depth, angle, justification menu, hidden flag, rigid flag, TeX flag, origin X/Y, font (picture menu: 35 PostScript + LaTeX), text field | size, pen colour, depth, angle, justification, flags, origin, font combo, text | fixed in W10; font chooser E11, canvas editing E10 (both accepted) |
| COMPOUND | comments; top-left corner (Return moves), bottom-right corner (Return moves), width/height (Return scales), minimum and maximum depth, **Collapse Depths**, number of objects, **one text field per text in it** | comments, depth ("Collapse depths to": sets every member), number of objects | partial: W10-15. Fixed in W10: the comment now belongs to the compound itself (it was written into every member) |
| Whole Figure | comments, Done, Cancel | same | new in W10 |

### Findings of group 3

| Id | Difference | Fixed or task |
|---|---|---|
| (fixed) | Position rows missing for every kind, units choice, arrow type and size per arrow, arc type, text hidden/rigid/TeX flags and origin, picture flip, cap/join only where xfig has them, width and radius ranges, Cancel after Apply, figure comments, shape-factor panel, Edit mouse hints | W10 group 3: `object.set_geometry` and extra `object.set_attributes` parameters (`geometry`, `pie`, `rigid`, `special`, `flipped`, `arrow_thickness`, `forward_arrow_spec`, `backward_arrow_spec`), `UiRequest::EditFigureComments` / `EditSplinePoint`; tests `crates/engine/tests/edit_panel.rs`, `ui_smoke.rs` |
| W10-15 | Compound panel: reposition by corner, rescale by width/height, minimum and maximum depth, Collapse Depths as a button, one text field per contained text | plan W10-15, gaps #31 |
| W10-16 | Picture panel: editable file name with Reread and Browse, type, size, colours and transparent colour, width/length fields that resize, rotation 0/90/180/270, h/w ratio, Shrink / Expand to orig, Use original size, Scale by %, Edit Image (external editor) and Screen Capture, XBM pen colour | plan W10-16, gaps #31 |
| W10-17 | Done as one undo step for all Applies; point panel drawing live; fill intensity limits for black and white (>100 clamps) | plan W10-17, gaps #31 |

xfig features with no equivalent that are not gaps: Screen Capture grabs an X11 screen region (platform specific; only the file route is needed), "Edit Image" runs an external editor named in a resource (part of W10-16).

### Proposed modernisations (group 3)

Not implemented. E4 (colour picker and editor), E10 (canvas text editing) and E11 (font chooser) are accepted and not repeated here. Each keeps every xfig function listed above.

1. **Live edit panel with preview.** xfig: nothing changes until Apply or Done. Proposal: changes show on the canvas as they are made (as the indicator does), Cancel undoes, Done is one undo step; kept: Done, Apply, Cancel, Return and Esc. Hours: 8 (includes W10-17).
2. **Edit several objects at once.** xfig: one object per panel, Update mode copies attributes. Proposal: the panel opens for the whole selection, shows common values and "mixed" where they differ, and changes only what is touched. Kept: single-object panel and Update. Hours: 10.
3. **Point table.** xfig: a fixed list of X/Y fields, shape factors through the right-button panel. Proposal: a table with add, delete and insert point rows, paste of coordinates, and a shape-factor column for X-splines (the right-button panel stays). Hours: 6.
4. **Fields that take units and expressions** ("2.5cm", "1/3 in", "3*1.5") with the unit shown. Kept: the Units menu. Hours: 3.
5. **Picture rows with a preview.** Thumbnail with size, colours and transparent colour, a missing-file marker with Relink, drag and drop of a file onto the panel. Kept: all W10-16 rows. Hours: 5.
6. **Pictorial pop-ups** for line style, cap, join and arrow type using the existing vector icons (`vector_icons.rs`), like xfig's picture menus but at any density. Kept: text names as tooltips. Hours: 3.

Total if all accepted: about 35 h besides W10-15 (5), W10-16 (8), W10-17 (3).

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
| 2026-10-21 | Created; group 1 compared from xfig's sources. |
| 2026-10-21 | Group 1 re-checked against captures of the real X11 xfig (menus, main window, Grid Mode pop-up); group 2 (Open, Save As, Merge, Export, Print) compared with captures; W10-4 to W10-10 added; merge as one compound, Figure Offset in the engine and the input-file check for export fixed. |
| 2026-10-21 | A11: group 1 menus re-compared with real screenshots of RustFig (`a11-*.png`); Unzoom/Zoom Out accelerator labels fixed. |
| 2026-10-21 | W11: main window look (3D buttons, mode panel fits, compact mouse box, rulers as xfig), screenshots `target/window-parity/w11-*.png`. |
| 2026-10-21 | Group 3 (object edit panels) compared with captures of the real X11 xfig; position rows, arrow rows, arc type, text flags, units, Cancel-restores, figure comments and the spline point panel added; W10-15 to W10-17 added. |
