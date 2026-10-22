# Preferences

Edit > Global settings holds RustFig's preferences; they are saved when you change them and loaded at every start (the panel shows where).

- **xfig keyboard shortcuts**: on by default; xfig's Meta and Ctrl keys ([Keys](keys.md)).
- **Show page borders**, **Show depth manager**, **Show info balloons** and the **balloon delay**: the View menu's switches.
- **Button style**: 3D (xfig's bevelled buttons) or flat.
- **Units and page**: opens Document settings.
- **xfig resources**: every X resource and command-line option of xfig under its xfig name (for example `startfontsize`, `startlinewidth`), with its value, default and what RustFig does with it. A value set here is kept for every start; xfig's command-line options override it for one run.
- **Use current attributes as defaults**: the indicator panel's values become the start values for new drawings.
- **Your colours**: colours added here appear in every colour menu of every drawing.

The colour theme follows the system's light or dark setting.

Document-specific settings (units, paper, orientation, comments) are saved in the drawing itself: see Edit > Document settings and [Zoom and view](view.md).

## Command-line options

RustFig starts like xfig: `rustfig [options] [file]`, and every option of xfig's manual page is accepted, so `rustfig -metric drawing.fig` does what `xfig -metric drawing.fig` does. An option changes the setting for that run only; your saved preferences stay as they are. Options take their value as the next word (`-startfontsize 16`) and may be shortened as long as the start is unique among xfig's options (`-me` for `-metric`). `-help` lists them all.

- Units and page: `-metric` (or `-centimeters`), `-inches` (or `-imperial`), `-landscape`, `-portrait`, `-paper_size A4`, `-magnification 80`.
- Start attributes: `-startfontsize`, `-startlinewidth`, `-startfillstyle`, `-startgridmode 0-3`, `-startposnmode 0-4`, `-startpsFont`, `-startlatexFont`, `-latexfonts`, `-specialtext`, `-hiddentext`, `-rigidtext`, the `-startarrow...` options.
- View: `-zoom 1.5`, `-showpageborder`, `-userscale`, `-userunit`, `-geometry 1000x700+50+50` (window size and position).
- Export: `-exportLanguage pdf`, `-export_margin`, `-correct_font_size`.
- `-update file.fig ...` rewrites old files in the current Fig format (the original stays as `file.fig.bak`) and quits without opening a window.
- Options that only make sense on X11 (`-display`, `-visual`, `-iconic`, colours and fonts of the toolkit) are accepted and ignored; RustFig prints a note to the terminal. An unknown option stops the start with a message.
- Resources in `~/.Xresources` or `~/.Xdefaults` (`Fig.startfontsize: 14`) are read too, and fill in settings your preferences file does not have.
