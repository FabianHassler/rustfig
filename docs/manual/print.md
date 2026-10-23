# Print

File > Print (Meta+P, Ctrl+P) opens the Print panel. RustFig makes a PDF of the page exactly as fig2dev lays it out and hands it to the operating system's print system: you choose printer, copies and page ranges there.

## The panel

- **Magnification** and **Fit to Page** (a half inch of margin all round); **Figure size** follows the magnification. The magnification, **Paper size**, **Orientation**, **Justification** (centred or flush left) and **Pages** (single or multiple, with **Overlap**) are the figure's own settings, stored in the .fig file as in xfig and shared with the Export panel.
- **Background** colour, **Grid** (minor and major spacing), **Layers**: print all, or only the active ones (depths not switched off in the depth manager), optionally with the page sized by them.
- **Printer** and **Print job params** (options for `lp`, for example `-n 2`; `%f` is the figure's file name) apply when **The system's print dialog** is switched off: the PDF then goes straight to the printer named (macOS and Linux `lp`, Windows the Print verb). With it on (the default) the PDF opens in the system's viewer (Preview on macOS, the desktop's PDF viewer on Linux, the default viewer on Windows) and you print from there.
- **Preview** shows the page as it prints, made again when you change an option; with several pages, step with the arrow buttons. Very large figures are not previewed.
- **Print FIGURE** prints. **Add to Batch** keeps the figure's PDF in the batch (with the settings it has now), **Print Batch** sends every figure of the batch, one after the other, and **Clear Batch** forgets them. The batch lives in memory until RustFig quits.
- Shift+Meta+P (Quick print) prints with the kept settings without the panel; if figures wait in the batch it prints the batch instead, as xfig does.

The panel's values are kept in the settings file. In the web app printing is not available yet: export to PDF and print that.

## Scripts

`print.settings`, `print.pdf` (the PDF as base64), `print.run`, `print.quick`, `print.batch_add`, `print.batch_clear`, `print.batch_print`. Without a front end (command line, MCP) the engine has no print system attached and printing reports that.
