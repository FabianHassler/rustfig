# Print

File > Print (Meta+P, Ctrl+P) opens the Print panel. RustFig makes a PDF of the page exactly as fig2dev lays it out and shows your operating system's own print dialog for it: you choose printer, copies, page ranges and the printer's options there (the macOS print panel, the Windows print dialog, the desktop's print dialog on Linux).

## The panel

- **Magnification** and **Fit to Page** (a half inch of margin all round); **Figure size** follows the magnification. The magnification, **Paper size**, **Orientation**, **Justification** (centred or flush left) and **Pages** (single or multiple, with **Overlap**) are the figure's own settings, stored in the .fig file as in xfig and shared with the Export panel.
- **Background** colour, **Grid** (minor and major spacing), **Layers**: print all, or only the active ones (depths not switched off in the depth manager), optionally with the page sized by them.
- **Preview** shows the page as it prints, made again when you change an option; with several pages, step with the arrow buttons. Very large figures are not previewed.
- **Print FIGURE** opens the system's print dialog with the page as the preview shows it; if you cancel the dialog nothing is printed. **Add to Batch** keeps the figure's PDF in the batch (with the settings it has now), **Print Batch** sends all figures as ONE print job (their PDFs joined into one multi-page PDF, one print dialog; cancel keeps the batch), and **Clear Batch** forgets them. The batch lives in memory until RustFig quits.
- Shift+Meta+P (Quick print) prints with the kept settings without the panel; if figures wait in the batch it prints the batch instead, as xfig does.

The panel's values are kept in the settings file. RustFig has no printer name or job options of its own: the dialog lists the printers the system knows.

- macOS: the standard print panel (it also saves as PDF). Windows: the print dialog; the PDF then goes to the chosen printer through the program registered for PDF files, so one must be installed, and the dialog's copies and page ranges are not passed on. Linux: the xdg-desktop-portal print dialog (needs a portal with a Print backend, such as the GTK or KDE one). Web: the PDF loads in a hidden frame and the browser's print dialog opens for it; if the browser refuses that (some PDF viewers do), the PDF opens in a new tab instead, where you print it with the viewer (allow pop-ups).

## Scripts

`print.settings`, `print.pdf` (the PDF as base64), `print.run`, `print.quick`, `print.batch_add`, `print.batch_clear`, `print.batch_print`. Without a front end (command line, MCP) the engine has no print system attached and printing reports that.
