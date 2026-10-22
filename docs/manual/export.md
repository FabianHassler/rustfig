# Export

File > Export (Meta+X, Ctrl+E) writes the drawing in another format. RustFig has no separate print dialog yet: export to PDF and print that.

## The Export panel

- **Language** (format): the list holds xfig's export languages. Vector formats keep vectors: PDF, SVG, EPS and PostScript, the LaTeX and TeX-family languages (LaTeX picture, epic, eepic, PiCTeX, TikZ, PSTricks, pict2e, pstex and pdftex pairs, and others), MetaPost, METAFONT, CGM, HPGL, DXF, EMF, Gerber, Tk and more. Bitmap formats: PNG, GIF, JPEG, TIFF, PPM, PCX, XBM, XPM. Entries still being ported are greyed with their plan task.
- **Magnification**: scales the output (percent).
- **Border**: extra space around the drawing, in Fig units.
- **Resolution** (bitmaps only) and **smoothing** (none, some, more).
- The paper, orientation and further options of fig2dev (depth range, background, grid and so on) are in the panel for the languages that use them.
- **Export** writes the file next to the figure with the chosen name. **Export now** (Shift+Meta+X) repeats the last export without the panel.
- **Export on save**: when checked, every Save also rewrites the exported file, so the PDF in your document is always current.

## Notes

- Text uses fig2dev's font metrics; special LaTeX text (the "special" flag) is typeset by TeX in the TeX languages and shown as written on the canvas.
- Pictures in the figure are embedded, linked or converted as the format allows; EPS pictures on the canvas and in bitmaps need ghostscript (optional).
- The same exports are available from the command line: `rustfig-cli convert in.fig out.pdf`, `rustfig-cli formats`, or as `fig2dev` ([Automation](automation.md)).
