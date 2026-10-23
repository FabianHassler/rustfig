# Preferences

Edit > Global settings holds RustFig's preferences; they are saved when you change them and loaded at every start (the panel shows where).

The upper part is xfig's panel, with the same rows. **Ok** applies every row you changed; **Cancel** (or closing the window) throws the changes away. Each row is a command (`prefs.set`, the View toggles) and is kept in the preferences:

- **Autorefresh figure**: reload the figure when its file changes on disk (View > Autorefresh mode).
- **Track mouse in rulers**: the markers in the rulers follow the pointer (resource `trackCursor`).
- **Show page borders**, **Show depth manager**, **Show info balloons** and the **balloon delay**: the View menu's switches.
- **Show line lengths**: the length triangle while drawing lines (View > Show line lengths).
- **Show vertex numbers**: numbers the points of lines, polygons and splines on the canvas (View > Show vertex numbers, resource `shownums`).
- **Allow negative coords**: when off, points are kept right of and below the origin and the view moves to the origin (`allownegcoords`).
- **Draw axis lines**: pink lines through the origin (`showaxislines`).
- **Freehand drawing resolution**: the least distance, in Fig units (1/1200 inch), between two points of a freehand stroke (`freehand_resolution`, default 25).
- **Recently used files**: 0 to 9 entries in File > Open recent (`max_recent_files`).
- **Maximum image colors**: kept for pictures (`max_image_colors`; 0 is no limit); RustFig shows pictures in full colour, so it has no effect yet.
- **Image editor**: the program Edit Image starts (`image_editor`); `%f` stands for the picture file, and without `%f` the file is added at the end. The default (`open` on macOS, `start` on Windows, `xdg-open` on Linux) uses the system's own program for the file type.
- **Spell checker**, **HTML Browser**, **PDF Viewer**: kept for the features that use them (`spellcheckcommand`, `browser`, `pdf_viewer`). `%f` stands for a file with the drawing's texts; the spell checker prints the misspelt words, one per line. The default is `aspell list < %f` (xfig's was `spell %f`, a Unix program most systems no longer have); `hunspell -l %f` works the same way. If `aspell` is not installed, `hunspell -l %f` is tried; the Edit menu's Spell check uses it (see [Draw text](draw-text-pictures.md)).

The lower part is RustFig's own:

- **xfig keyboard shortcuts**: on by default; xfig's Meta and Ctrl keys ([Keys](keys.md)).
- **Button style**: 3D (xfig's bevelled buttons) or flat.
- **Theme**: System (follows the system's light or dark setting), Light or Dark. The drawing's paper stays white in every theme.
- **UI scale**: 75 % to 200 % (a slider in steps of 5), for small or large screens. Both are kept for every start (`app.theme`, `app.ui_scale`).
- **Units and page**: opens Document settings.
- **xfig resources**: every X resource and command-line option of xfig under its xfig name (for example `startfontsize`, `startlinewidth`), with its value, default and what RustFig does with it. A value set here is kept for every start; xfig's command-line options override it for one run.
- **Use current attributes as defaults**: the indicator panel's values become the start values for new drawings.
- **Your colours**: colours added here appear in every colour menu of every drawing.

The colour theme follows the system's light or dark setting unless you choose Light or Dark above.

Document-specific settings (units, paper, orientation, comments) are saved in the drawing itself: see Edit > Document settings and [Zoom and view](view.md).

## LaTeX text

Under **LaTeX text** in Edit > Global settings: **Typeset LaTeX text on the canvas** (View > Typeset LaTeX text), the **TeX program** (empty: `pdflatex`, then `lualatex`, from the `PATH`; the line below names the one in use) and the **Preamble** added after `\usepackage{amsmath,amssymb,color,graphicx}`. Each field is saved, and the texts typeset again, when you leave it. See [Text and pictures](draw-text-pictures.md).

## Command-line options

RustFig starts like xfig: `rustfig [options] [file]`, and every option of xfig's manual page is accepted, so `rustfig -metric drawing.fig` does what `xfig -metric drawing.fig` does. An option changes the setting for that run only; your saved preferences stay as they are. Options take their value as the next word (`-startfontsize 16`) and may be shortened as long as the start is unique among xfig's options (`-me` for `-metric`). `-help` lists them all.

- Units and page: `-metric` (or `-centimeters`), `-inches` (or `-imperial`), `-landscape`, `-portrait`, `-paper_size A4`, `-magnification 80`.
- Start attributes: `-startfontsize`, `-startlinewidth`, `-startfillstyle`, `-startgridmode 0-3`, `-startposnmode 0-4`, `-startpsFont`, `-startlatexFont`, `-latexfonts`, `-specialtext`, `-hiddentext`, `-rigidtext`, the `-startarrow...` options.
- View: `-zoom 1.5`, `-showpageborder`, `-userscale`, `-userunit`, `-geometry 1000x700+50+50` (window size and position).
- Export: `-exportLanguage pdf`, `-export_margin` (the border before any is kept), `-correct_font_size`. The Export panel's values are kept per language in the `[rustfig.export.<language>]` tables of the settings file.
- Libraries: `-library_dir FOLDER` (your library folder or a list file naming several folders, default `~/xfiglib`; [Libraries](libraries.md)), `-icon_view` / `-list_view`, `-library_icon_size N` (40 to 120).
- Files: `-write_bak` / `-nowrite_bak` (keep the old file as `NAME.bak` when saving; on by default).
- `-update file.fig ...` rewrites old files in the current Fig format (the original stays as `file.fig.bak`) and quits without opening a window.
- Options that only make sense on X11 (`-display`, `-visual`, `-iconic`, colours and fonts of the toolkit) are accepted and ignored; RustFig prints a note to the terminal. An unknown option stops the start with a message.
- Resources in `~/.Xresources` or `~/.Xdefaults` (`Fig.startfontsize: 14`) are read too, and fill in settings your preferences file does not have.
