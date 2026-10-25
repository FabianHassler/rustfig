# Files

RustFig's own formats are xfig's **Fig** files (`.fig`, Fig 3.2 on writing) and `.rustfig` (a lossless JSON version of the same drawing). RustFig reads Fig 1.3 to 3.2, so old figures open too; warnings appear in the message panel where an old file needed repairs.

- **New** starts an empty drawing; **Open** opens a file; **Merge** adds a file's objects to the current drawing. If the drawing has unsaved changes, RustFig asks first.
- **Open with preview...** and **Merge with preview...** (File menu, commands `app.open_panel` and `app.merge_panel`; native versions only) show xfig's file panel: the files of the **Current Dir** that match the **Filename Mask** (blank-separated patterns with `*`, `?` and `[set]`; the default is `*.fig *.rustfig *.xfig *.fig.gz *.fig.Z`), the **Directories** to go into (`..` goes up, **Home**, **Show Hidden**, **Rescan**), and the **Filename** field (Return opens or merges it; **Browse...** uses the system's own file dialog). Clicking a file previews it on the right: the figure drawn small, its size (in the open figure's units), the number of objects and the **Figure comments**. **Open** loads the file as File > Open does (asking first about unsaved changes); **Merge** adds it as one compound, moved by the **Figure Offset** X and Y, each in Inches, cm or Fig units (`file.merge` with `offset_x`, `offset_y` and `unit`). Scripts can ask for the list and the preview with `file.list_dir` and `file.preview`.
- **Open** also takes a PDF, SVG or PNG exported with **Embed the figure** ([Export](export.md)) and opens the figure inside it, under a `.fig` name next to the file; the same goes for dropping such a file on the window. Other PDF, SVG or PNG files give "no embedded figure".
- **Save** and **Save As** write the file. When they replace an existing file, the old one is kept as `NAME.bak` (xfig's `write_bak`; switch it off with the `write_bak` preference or `-nowrite_bak`).
- **Open** of a name that does not exist yet starts an empty figure with that name (xfig's "new file"); the first Save creates the file. While a compound is open ([Compounds](compounds.md)) a query asks whether to save the visible part or all.
- **Import picture** places an image file in the drawing ([Text and pictures](draw-text-pictures.md)).
- The most recent files are listed at the end of the File menu and open with their number key.
- Click the **file name** in the top strip to rename the file.
- **Save As** into another folder keeps pictures working: relative picture names are rewritten so they still point at the same files (xfig's `relative_path`); absolute names stay as they are. Undo after the save brings back the old names; save again to rewrite them.
- **Digitize...** (Alt+Z) saves the points of the selected polylines and splines as text for a plotting program, xfig's Digitize the other way round. Name the files by prefix, starting number and suffix (`PREFIX_NNNN.SUFFIX`, four digits, as xfig's example line shows); **New file for each line** writes one numbered file per object, **Append to one file** adds all lines to a single file (a blank line between lines, added to the file if it exists). A suffix of `csv` writes comma separated values with an `x_fig,y_fig` header, any other suffix one `x y` pair per line. Without calibration the numbers are figure units (1/1200 inch). Tick **Calibrate to data units** and give two reference points of a scanned plot (position in the figure and the value there) to get the plot's own units, with optional logarithmic axes. Boxes, arcs and pictures are not lines and are skipped. Command `file.digitize`; native versions only.
- **New window...** starts a second RustFig, optionally on a file (xfig's New xfig button); native versions only.
- **Autorefresh mode** (View menu, off at start, kept in the preferences): once a second RustFig looks at the open file and reloads it when another program changed it. If the figure has unsaved edits you are asked first: Reload (your edits are lost) or Keep my version. Saving from RustFig itself never triggers it. Native versions only.
- **Print** (Meta+P): the Print panel with page preview ([Print](print.md)).

## Autosave and recovery

While you work, RustFig writes a recovery copy of any drawing with unsaved changes a few seconds after the last change. If the program stops without saving (a crash, power loss, a killed process), the next start offers to restore each drawing it finds, or discard it. After a normal save or exit the copy is removed. Restore a drawing, check it and save it where you want.

In the browser the copy goes to the browser's local storage (as Fig text, drawings up to 2 MB). Reload the page and RustFig offers to restore it or discard it; saving the drawing or starting a new one removes the copy. Clearing the site's data, or a private window, loses it, so download (File > Save) what matters.

**Saved versions.** Each time you save, RustFig keeps the file it replaces in the recovery folder. File > Saved versions… lists the last versions of the open file with their times; Restore puts one back as the open drawing (save it to keep it). Global settings > Saved versions kept sets how many (default 5, 0 switches it off, at most 50). Versions are kept only where the recovery folder is available (native app). The commands are `file.versions` and `file.restore_version`.

## Starting from the command line

Running `rustfig FILE.fig` opens a file at start. See [Automation](automation.md).

## Opening from the file manager (macOS)

Double-clicking a `.fig` file in Finder, choosing Open With > RustFig, `open -a RustFig FILE.fig` or dropping a file on the Dock icon opens it, whether RustFig is already running or is started by it. As with File > Open, RustFig asks before it discards unsaved changes; several files sent at once open one after the other.

## The web version

RustFig also runs in a browser (WebAssembly). Build it with `cargo xtask web` (needs `trunk`: `cargo install --locked trunk`) and serve the `dist/` folder with any static web server, or run `cargo xtask web --serve` and open http://127.0.0.1:8080. The page shows "Loading RustFig…" until the program has started.

In the browser there are no file paths. **File > Open** shows the browser's file picker (a Fig file, or a PDF, SVG or PNG export with the figure embedded); you can also drop such a file on the window. **File > Save** and **Save As** download the figure as a file named after the open one (`figure.fig` for a new figure; the browser decides the folder), and **File > Export** downloads each exported file. The menu leaves out what only makes sense on the desktop: Open and Merge with preview, Digitize, Capture Screen, New window, Saved versions and Exit.

**File > Merge** (Alt+M) shows the same picker and adds the picked figure as one compound, as on the desktop.

**Pictures in the browser.** Picture mode, Import picture and the picture panel's Browse/Relink show the browser's picker too (a figure's pictures can also be dropped on the picture panel). The browser gives RustFig the picture's bytes, not a place on disk, so RustFig keeps them in memory under the file name for as long as the page stays open, and the figure stores only that name, as always. After you reload the page, or when you open a `.fig` that names pictures, the pictures are empty boxes with their names until you pick them again (Reread in the picture panel, or Relink: pick the file with the same name). Saving the figure downloads only the `.fig`; keep the picture files next to it to see them on the desktop. Exports made in the browser (File > Export) do not contain these pictures yet. EPS, PDF and compressed pictures need the desktop version: the browser build reads PNG, JPEG, GIF, BMP, PNM and TIFF.

**Libraries in the browser.** Library mode works with the bundled libraries, which the page serves as files next to the program (see [Libraries](libraries.md)). The Source line reads "bundled libraries" (there is no folder to choose).
- **Capture Screen...** (File menu, command `app.capture`; xfig's Screen Capture button; not in the browser) grabs a window or an area of the screen into a picture. RustFig's window steps aside (it is minimised) while the system's own selector runs: on macOS `screencapture` (drag an area, press Space to pick a window, Escape cancels; RustFig needs the Screen Recording permission in System Settings > Privacy & Security), on Linux the desktop's Screenshot portal, on Windows the whole screen (no selector). Then a Save dialog asks for the PNG file name (default `<figure>_<seconds>.png`, as xfig named its captures, next to the figure); the image is written there and added to the drawing as a picture object (like **Import Picture...**) at the centre of the view, ready to move and resize. Cancelling the selector or the Save dialog adds nothing.
