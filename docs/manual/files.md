# Files

RustFig's own formats are xfig's **Fig** files (`.fig`, Fig 3.2 on writing) and `.rustfig` (a lossless JSON version of the same drawing). RustFig reads Fig 1.3 to 3.2, so old figures open too; warnings appear in the message panel where an old file needed repairs.

- **New** starts an empty drawing; **Open** opens a file; **Merge** adds a file's objects to the current drawing. If the drawing has unsaved changes, RustFig asks first.
- **Save** and **Save As** write the file. When they replace an existing file, the old one is kept as `NAME.bak` (xfig's `write_bak`; switch it off with the `write_bak` preference or `-nowrite_bak`).
- **Open** of a name that does not exist yet starts an empty figure with that name (xfig's "new file"); the first Save creates the file. While a compound is open ([Compounds](compounds.md)) a query asks whether to save the visible part or all.
- **Import picture** places an image file in the drawing ([Text and pictures](draw-text-pictures.md)).
- The most recent files are listed at the end of the File menu and open with their number key.
- Click the **file name** in the top strip to rename the file.
- **Save As** into another folder keeps pictures working: relative picture names are rewritten so they still point at the same files (xfig's `relative_path`); absolute names stay as they are. Undo after the save brings back the old names; save again to rewrite them.
- **Digitize...** (Alt+Z) saves the points of the selected polylines and splines as text for a plotting program, xfig's Digitize the other way round. Name the files by prefix, starting number and suffix (`PREFIX_NNNN.SUFFIX`, four digits, as xfig's example line shows); **New file for each line** writes one numbered file per object, **Append to one file** adds all lines to a single file (a blank line between lines, added to the file if it exists). A suffix of `csv` writes comma separated values with an `x_fig,y_fig` header, any other suffix one `x y` pair per line. Without calibration the numbers are figure units (1/1200 inch). Tick **Calibrate to data units** and give two reference points of a scanned plot (position in the figure and the value there) to get the plot's own units, with optional logarithmic axes. Boxes, arcs and pictures are not lines and are skipped. Command `file.digitize`; native versions only.
- **New window...** starts a second RustFig, optionally on a file (xfig's New xfig button); native versions only.
- **Autorefresh mode** (View menu, off at start, kept in the preferences): once a second RustFig looks at the open file and reloads it when another program changed it. If the figure has unsaved edits you are asked first: Reload (your edits are lost) or Keep my version. Saving from RustFig itself never triggers it. Native versions only.
- **Print**: not available yet. Export to PDF ([Export](export.md)) and print that.

## Autosave and recovery

While you work, RustFig writes a recovery copy of any drawing with unsaved changes a few seconds after the last change. If the program stops without saving (a crash, power loss, a killed process), the next start offers to restore each drawing it finds, or discard it. After a normal save or exit the copy is removed. Restore a drawing, check it and save it where you want.

## Starting from the command line

Running `rustfig FILE.fig` opens a file at start. See [Automation](automation.md).
