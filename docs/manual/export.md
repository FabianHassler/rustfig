# Export

File > Export (Meta+X, Ctrl+E) writes the drawing in another format. To print, use File > Print ([Print](print.md)).

## The Export panel

- **Language** (format): the list holds xfig's export languages. Vector formats keep vectors: PDF, SVG, EPS and PostScript, the LaTeX and TeX-family languages (LaTeX picture, epic, eepic, PiCTeX, TikZ, PSTricks, pict2e, pstex and pdftex pairs, and others), MetaPost, METAFONT, CGM, HPGL, DXF, EMF, Gerber, Tk and more. Bitmap formats: PNG, GIF, JPEG, TIFF, PPM, PCX, XBM, XPM. Entries still being ported are greyed with their plan task.
- **Magnification** scales the output (percent). It is the figure's own setting and is saved in the .fig file when you export. **Fit to Page** sets the magnification so that the figure fills the paper (a half inch of margin all round); **Figure size** shows the size of the result in the figure's unit (inches or cm) and follows the magnification.
- The panel shows only the rows a language can use, as xfig's does:
  - **Border** (points; pixels for bitmaps): blank space around the figure. xfig's `export_margin` resource is its start value.
  - **Background**: a colour behind the figure (the default is none). Grid: **Minor** and **Major** spacing draw a light grid under the figure (PostScript, PDF and bitmaps).
  - **EPS/PS preview** (EPS and PostScript): none, ASCII (EPSI), colour TIFF or monochrome TIFF.
  - **PDF page mode** (PDF): a whole page of the paper size instead of a figure-sized PDF. The paper rows below are greyed unless it is on.
  - **Paper size**, **Orientation**, **Justification**, **Pages** (single or multiple, with **Overlap**) and the **Offset** X and Y (in, cm or Fig units): the figure's page settings (PostScript and PDF page mode). They are stored in the .fig file, as in xfig.
  - **GIF transparent color**: none, the background, or a colour of the figure. **Image quality** (JPEG). **Resolution** and **Smoothing** (bitmaps).
- **Preview** shows a small picture of the result, made again when you change the figure or an option (not for very large figures).
- The values of these rows are kept for each language in the settings file (`[rustfig.export.<language>]`, see [Preferences](preferences.md)), never in the drawing; **Reset to xfig defaults** puts the language's values back (no border, no grid, no preview, no smoothing).
- The same values are available to scripts as `export.settings`, and `file.export` with `"panel": true` uses them.
- **Export** writes the file next to the figure with the chosen name. **Export now** (Shift+Meta+X) repeats the last export without the panel.
- **Export on save**: when checked, every Save also rewrites the exported file, so the PDF in your document is always current.

- **Embed the figure** (PDF, SVG and PNG; off at start, kept per language like the other rows): stores the figure's Fig text inside the file, so you can edit the picture you sent around. In a PDF it is an attached file `figure.fig` (PDF readers list it among the attachments); in an SVG it is a `<metadata>` element; in a PNG a compressed text chunk (`rustfig-fig`). Other programs show the file as usual and ignore the payload. **File > Open** (or dropping the file on the window) opens the embedded figure under a `.fig` name, so Save never overwrites the export. The embedded figure includes depths that are switched off and the figure's comments, so leave the option off for files you publish. `file.export` takes `"embed": true`; Export on save keeps it.

## Exporting in the background

Export from the panel runs on a background thread, so the window stays responsive. A progress window shows a spinner and the seconds elapsed, with **Cancel**. Cancel is checked between steps: a driver that is already running finishes, but its result is discarded and no file is written. When the export is done an "Export finished" notice names the file and offers **Show in folder**. The commands `app.cancel_export` and `app.show_export_folder` do the same from scripts. `file.export` (commands, MCP, the command line) is synchronous. The web app downloads the file at once.

## Notes

- Text uses fig2dev's font metrics; special LaTeX text (the "special" flag) is typeset by TeX in the TeX languages and shown as written on the canvas.
- Pictures in the figure are embedded, linked or converted as the format allows; EPS pictures on the canvas and in bitmaps need ghostscript (optional).
- The same exports are available from the command line: `rustfig-cli convert in.fig out.pdf`, `rustfig-cli formats`, or as `fig2dev` ([Automation](automation.md)).
