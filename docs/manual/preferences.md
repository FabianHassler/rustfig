# Preferences

Edit > Global settings holds RustFig's preferences; they are saved when you change them and loaded at every start (the panel shows where).

The window has a **filter** field at the top and five tabs: **Window**, **Drawing**, **Files**, **Programs** and **Advanced**. **Ok** applies every row you changed; **Cancel** (or closing the window) throws the changes away, as in xfig. Switch on **Apply changes immediately** (off by default; kept in the preferences) to have each change take effect as you make it. Each row is a command (`prefs.set`, the View toggles) and is kept in the preferences.

**Filter**: type part of a row's label, an xfig resource name (`shownums`) or a command-line option (`-notrack`), in any case. The tabs give way to the matching rows of every tab, grouped under the tab's name, so nothing is hidden by the tab you were on. Clear the field to get the tabs back.

**Advanced** lists all of xfig's resources (see below) as editable fields: a check box for yes/no resources, a field for numbers and texts (empty means xfig's default; a value that does not read as a number is shown in red and not applied), with the command-line option beside each. Only resources that change something in RustFig are listed; the others (for example `canvasbackground` or `grid_unit`) are still accepted on the command line and in `~/.Xdefaults`, and stay in your settings file, but are not shown.

**Defaults** (next to Cancel and Ok) sets every row of the window back to xfig's defaults (RustFig's own for its rows); like any other change, Ok applies it and Cancel discards it. Scripts use `prefs.reset` (optionally with `names`, a list of resource names). Your colours, recent files and saved styles are not touched.

The rows of xfig's panel are spread over the tabs: **Window** has the display switches (autorefresh, rulers, page borders, depth manager, balloons, line lengths, vertex numbers, crosshair, negative coordinates, axis lines) and RustFig's look; **Drawing** has the freehand resolution, the current-attributes button, LaTeX text and your colours; **Files** has the recent files and saved versions; **Programs** has the image editor and the spell checker. Each row:

- **Autorefresh figure**: reload the figure when its file changes on disk (View > Autorefresh mode).
- **Track mouse in rulers**: the markers in the rulers follow the pointer (resource `trackCursor`).
- **Show page borders**, **Show depth manager**, **Show info balloons** and the **balloon delay**: the View menu's switches.
- **Show line lengths**: the length triangle while drawing lines (View > Show line lengths).
- **Show vertex numbers**: numbers the points of lines, polygons and splines on the canvas (in red, from 0) (View > Show vertex numbers, resource `shownums`).
- **Allow negative coords**: when off, points are kept right of and below the origin and the view moves to the origin (`allownegcoords`).
- **Draw axis lines**: pink lines through the origin (`showaxislines`).
- **Freehand drawing resolution**: the least distance, in Fig units (1/1200 inch), between two points of a freehand stroke (`freehand_resolution`, default 25).
- **Recently used files**: 0 to 9 entries in File > Open recent (`max_recent_files`).
- **Image editor**: the program Edit Image starts (`image_editor`); `%f` stands for the picture file, and without `%f` the file is added at the end. The default (`open` on macOS, `start` on Windows, `xdg-open` on Linux) uses the system's own program for the file type.
- **Spell checker**: the command Spell check runs (`spellcheckcommand`). `%f` stands for a file with the drawing's texts; the spell checker prints the misspelt words, one per line. The default is `aspell list < %f` (xfig's was `spell %f`, a Unix program most systems no longer have); `hunspell -l %f` works the same way. If `aspell` is not installed, `hunspell -l %f` is tried; the Edit menu's Spell check uses it (see [Draw text](draw-text-pictures.md)).

RustFig's own rows (Window tab, except as noted):

- **xfig keyboard shortcuts**: on by default; xfig's Meta and Ctrl keys ([Keys](keys.md)).
- **Button style**: 3D (xfig's bevelled buttons) or flat.
- **Theme**: System (follows the system's light or dark setting), Light or Dark. The drawing's paper stays white in every theme.
- **UI scale**: 75 % to 200 % (a slider in steps of 5), for small or large screens. Both are kept for every start (`app.theme`, `app.ui_scale`).
- **Units and page**: opens Document settings.
- **xfig resources** (the Advanced tab; **Open as window…** there opens the table in a window of its own): every X resource and command-line option of xfig under its xfig name (for example `startfontsize`, `startlinewidth`), with its value, default and command-line option (only the ones RustFig uses). A value set here is kept for every start; xfig's command-line options override it for one run.
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
- `-scale_factor 2` scales every file by 2 as it is read (opening, merging and `-update`): positions, sizes and arrowheads, not font sizes or line widths. With `-update` it rescales the files for good.

### Coming from xfig

On the first start RustFig takes the recent files of xfig's `~/.xfigrc` into its File menu (and xfig's `max_recent_files`). On Linux, `install.sh` also installs an `xfig` command that starts RustFig, when no xfig is installed; with Homebrew, `brew install rustfig --with-xfig` does the same (it cannot be installed next to Homebrew's `xfig`). Scripts that call `xfig file.fig` then open RustFig.
- Options that only make sense on X11 (`-display`, `-visual`, `-iconic`, colours and fonts of the toolkit) are accepted and ignored; RustFig prints a note to the terminal. An unknown option stops the start with a message.
- Resources in `~/.Xresources` or `~/.Xdefaults` (`Fig.startfontsize: 14`) are read too, and fill in settings your preferences file does not have.

## Accessibility

Every button, check box, field and ruler has a name that screen readers (VoiceOver, Narrator, Orca) announce through AccessKit; the mode and attribute buttons read as their xfig tooltips, the colour swatches by colour name. The canvas reads as a summary of the drawing, for example "Canvas: 3 objects: polyline, box, text 'Hello'" (the first eight objects, then "and N more"). A test checks that no interactive widget of the main window has an empty name; the check with a real screen reader is manual.
