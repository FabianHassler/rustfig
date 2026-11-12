# Automation

Everything RustFig does is a **command** with an id (for example `shape.rectangle`, `transform.rotate`, `tool.chop`). The menus, keys and mode buttons all run commands, and so can programs.

- **The command list**: Help > RustFig Reference shows every command with its id, parameters and shortcut. It is also the reference for the other routes.
- **Control channel**: start with `rustfig --control 7979 FILE.fig` and send JSON lines to `127.0.0.1:7979`: `{"id":1,"method":"engine.execute","params":{"command":"shape.rectangle","params":{"x":600,"y":600,"width":1200,"height":600}}}`. Other methods list commands, run user-interface commands such as `help.open`, report the window state or take a screenshot. `--background` opens the window without taking the keyboard focus.
- **MCP**: `rustfig-cli mcp` is a Model Context Protocol server over standard input and output, so an AI assistant can draw and edit figures with the same commands, and render the result. Setting it up for Claude Code, Claude Desktop, Codex or Cursor, with examples, is described in [AI assistants](ai-assistants.md).
- **Command line**: `rustfig-cli convert in.fig out.pdf` converts files. Its output depends only on the file and the command line, never on your settings: it writes what File > Export writes with the panel's defaults (Reset to xfig defaults), the figure's own LaTeX preamble, and TeX from the `PATH` (`--tex PROGRAM` names another). Flags named after the panel's rows set a row (`--border`, `--background`, `--grid`, `--preview`, `--page-mode`, `--offset`, `--overlap`, `--dpi`, `--smoothing`, `--quality`, `--transparent`, `--embed`, `--latex`/`--no-latex`, `--hpgl-font`, `--hpgl-pcl`, `--asy-splines`), and `rustfig-cli formats LANG` lists the flags a language takes. Over MCP, `file.export` with `"panel": true` (or `{…}` with changes) exports the same way. `rustfig-cli info FILE` describes one, and `rustfig fig2dev ARGS` (or running the program under the name `fig2dev`) is fig2dev's own command line (`fig2dev -L eps in.fig out.eps`), whose options are documented in fig2dev's manual page; `fig2dev -h` lists the general options, `fig2dev -h -L eps` adds those of one language.

Coordinates in commands are Fig units: 1200 per inch, 450 per centimeter in metric drawings, y growing downwards. Mutating commands are atomic and one undo step each. A parameter a command does not take is not an error: the command runs, and its reply's `warnings` name each parameter it ignored (a misspelt name, or `color` where the command wants `pen_color`).

**Names**: object ids are session numbers and change when a file is opened. Give an object a name with `object.set_attributes {"ids":[7],"name":"legend"}` (1 to 100 characters; `""` removes it): the name is stored in the .fig as a comment line `rustfig-name: legend`, which xfig and fig2dev ignore, and `object.list` shows it. Every command that takes `ids` also takes `"names": ["legend"]` (alone or together with `ids`). Names need not be unique: a name selects every top-level object carrying it, so copies of a named object answer to it too; an unknown name is an error.

To open a manual page from a script: `{"method":"ui.run","params":{"command":"help.open","params":{"topic":"chop.md"}}}`.

The sections below list the commands the other topics leave out, by area. Parameters are named as in the Command list.

## Drawing

- `shape.polyline`, `shape.spline`, `shape.arc`: the parameter `mid_arrows: [fractions of the line's length]` puts any number of arrowheads in the middle of the line (see Arrow Mode in [Indicator panel](attributes.md)).
- `shape.dimension_line`: draws a dimension line. `label_along` leaves out the box and runs the length along the line. The settings are the indicator's `dimline` group (thickness, arrows, ticks, box and text colors, font, decimals, fixed text).
- `object.recompute_dimension`: recomputes the length, box and ticks of a dimension line.
- `tool.point`: xfig's keyboard point entry. It gives the next point as text instead of a click: `1.5, 2` (ruler units; fractions as `1-1/2`), `r1, r-0.5` or `r1 0.5` (relative to the current point: the last point of the line, or where the operation started), `a` for absolute, or polar `2<30` (length and angle in degrees, `2<0.5p` in multiples of pi, `2<1r` in radians; from the current point). The point is used exactly, without snapping.
- `shape.text`: `box` is `true`, or `{margin, fill_color, area_fill, pen_color, thickness}`, and `anchor` sets the anchor. Without them the indicator's `text_boxed`, `text_box_margin` and `text_anchor` apply. Several lines made with `\n` share one box.
- `indicator.set`: `mid_arrow` sets the middle arrowhead; `grid` and `grid_iso` set the grid mode (xfig's resource `startgridmode`, 0 to 8); `num_x_copies` and `num_y_copies` (0 to 99) set the copy counts used by array placement.
- `view.toggle_lengths` and `view.toggle_crosshair`: Show Lengths and Show crosshair.

## Text and LaTeX

- `tool.event`: the text-editing keys also run through the control channel and `tool.event` as `superscript`, `subscript`, `kill_to_end`, `kill_to_start` and `kill_selection`.
- `text.set_local_font`: a font name or `null`, for the selection or `ids`; it also sets the font for new text. `text.local_fonts` lists the local fonts. `object.set_attributes` takes the `local_font` parameter.
- `text.insert_char`: inserts a character from the character map.
- `text.spell_check`: runs the spell checker (also over MCP).
- `edit.search_replace`: `find`, `replace`, `case_sensitive`, `whole_word`. `edit.search_step`: `direction` next or previous. `edit.show_object`: `id`.
- `doc.set_latex_preamble`: sets the LaTeX preamble of the figure (the Whole Figure dialog). `view.set_latex_preamble` sets the preamble new figures start with (Global settings). `doc.reapply_latex_preamble` applies that one to the open figure (undoable).
- `picture.store`: adds a picture to the figure's memory in the web version, where there are no file paths.
- `picture.resize`, `picture.rotate`, `picture.set_file`, `picture.info`, `picture.edit_image` and `picture.import_objects`: the actions of the picture's Edit panel (Reread, Relink, Import as objects and so on).

## Finding objects

- `tool.find` enters the Find mode.
- `find.set`: sets the criteria `kind`, `pen_color`, `fill_color`, `fill`, `depth`, `depth_min`, `depth_max`, `thickness`, `line`, `arrows`, `font`, `font_size`, `text`, `inside`; it also enters the mode.
- `find.toggle` (`id`), `find.state`, and `find.apply` (`action`: `update`, `depth` with `depth`, `delete` or `group`).
- `edit.find_objects` takes the same criteria, only reads and returns `ids`, to use with `object.update`, `arrange.set_depth`, `edit.delete` or `object.group`.

## Editing and transforms

- `edit.paste`: `dx` and `dy` place the objects at once, moved by that offset.
- `edit.copy_array`: the array placement of Copy (Num X Copies and Num Y Copies).
- `arrange.align`: `h` and `v`, each one of none, left, center, right, distribute_centres, distribute_edges, abut. `arrange.align_canvas` aligns every object to the page by the indicator's Vert Align and Horiz Align.
- `transform.nudge`: moves the selection by a small step (the nudge keys).
- `tool.trim`, `object.trim`, `object.split_at` and `object.extend`: the Trim, Split and Extend operations. `object.cut_all` cuts all selected objects against each other. To cut at chosen axe objects see [Chop](chop.md).
- `indicator.update_summary`: the line "Update will change: …" of Update mode.

## Colors and styles

- `color.*`: `color.parse`, `color.new`, `color.edit`, `color.delete`, `color.recent` and so on, for the user colors.
- `style.family_add`, `style.family_delete`, `style.family_rename`, `style.save`, `style.apply`, `style.rename`, `style.delete` and `style.list`, each with a `family` parameter: the saved styles.

## Measuring and annotation

- `object.measure_angle`, `object.measure_length`, `object.measure_area` and `object.measure_reset`: every number the measure modes report in the message panel.
- `object.annotate_angle`: draws the arc or right-angle mark with its label. Parameters: `points` or `id`, `x`, `y`; `mark` arc or right; `radius`; `label`.

## Files and export

- `file.open`, `file.merge`: also read PDF (with `page`, default 1), SVG and SVGZ files, and compressed figures (`.gz`, `.Z`, `.z`, `.bz2`, `.xz`). `file.pdf_pages` counts the pages of a PDF. In the browser `file.open_data` and `file.merge_data` take the file's content.
- `file.merge` takes `offset_x`, `offset_y` and `unit` for the Figure Offset.
- `app.open_panel` and `app.merge_panel`: the Open and Merge panels with preview. `file.list_dir` and `file.preview` give the file list and the preview.
- `file.digitize`: saves the points of the selected lines (native versions only).
- `app.capture`: Capture Screen (not in the browser).
- `file.versions` and `file.restore_version`: the saved versions of the open file.
- `file.export`: exports synchronously, also from the command line and over MCP. `"panel": true` uses the settings of the Export panel, which `export.settings` returns. `"typeset_latex": true` typesets the TeX texts. `"embed": true` embeds the figure.
- `app.export_now`: Export with current settings. `app.cancel_export` and `app.show_export_folder` cancel a running export and show the folder of a finished one.
- The Asymptote spline choice (guide, hobby or exact) is fig2dev's option `S`: `rustfig-cli convert` takes `--option S=hobby`.
- `print.settings`, `print.pdf` (the PDF as base64), `print.run`, `print.quick`, `print.batch_add`, `print.batch_clear` and `print.batch_print`: printing. Without a front end (command line, MCP) the engine has no print system attached and printing reports that.

## View and drawing area

- `view.set_integer_zoom`: the indicator's Zoom button with Integer zoom. `app.unit_panel` opens the Unit/Scale panel.
- `doc.set_bounding_box` (`x1`, `y1`, `x2`, `y2` in Fig units), `doc.bounding_box_standard`, `doc.bounding_box_content` and `tool.bounding_box`: the figure's own bounding box. `doc.info` reports the box.

## Libraries

- `library.list`, `library.source` (what `library_dir` names: a folder, or a list file with its entries), `library.use` (`path`: make a folder or list file the source).
- `library.search`: `query`, `limit`, `comments`; finds figures of all libraries by name and comments. It skips hidden libraries unless `include_hidden: true`.
- `library.favourite`: `library`, `name`, `on` marks, unmarks or toggles a favorite figure (`on: false` drops it). `library.favourite_library`: `library`, `on` does the same for a library. `library.hide_library`: `library`, `on` hides, shows or toggles a library. `library.list` says which libraries are favorites and which are `hidden`.
- `library.favourites`: `recent: true` lists the figures placed lately, `clear: true` empties that list; each entry says whether its file `exists`.
- `library.objects`: lists the figures of a library; `comments: true` adds each figure's comments.
- `library.place`: `library`, `name`, `x`, `y` or `orig`, `ops`. `library.hand` puts the object in hand as Select object does. `library.transform`, `library.save` and `library.panel`.

## Plots

- `plot.add`: draws a plot. The frame is `x`, `y`, `width`, `height`. Settings: `mode`, `expr`, `xt`, `yt`, `tmin`, `tmax`, `file`, `xcol`, `ycol`, `data`, `xmin`, `xmax`, `ymin`, `ymax`, `samples`, `frame`, `xticks`, `yticks`, `axes` (`"frame"`, `"origin"` or `"none"`; the old `false` turns box, ticks and numbers off), `axis_arrows`, `grid`, `grid_minor`, `grid_style` (`dotted`, `dashed`, `solid`). In `xticks` and `yticks` also `scale`, `comma`, `labels` (`[[1, "Mon"], [2, "Tue"]]`) and `rotate`. Also `log_x`, `log_y`, `xlabel`, `ylabel`, `latex` (the axis labels as LaTeX) and `latex_numbers` (the tick numbers as LaTeX math, `$-1.5$`). On the web, give the text of a data file as `data`.
- `plot.update` changes some settings of an existing plot, `plot.get` returns them, and `plot.panel` and `plot.edit` open the dialog.
- **Several curves**: `plot.add` takes `curves`, a list in which each curve has its own source (`expr`, `xt`/`yt`, a data `file`, `data` text or `points`) and style (`pen_color`, `thickness`, `line`, `style_val`; `line: "none"` draws markers only) and markers (`marker`: circle, square, triangle, diamond, cross or plus; `marker_size` in Fig units; `marker_fill`). `mode` can also be `polar` (`expr` in `t`), `implicit` (`expr`, an equation), `derivative`/`integral` (`of`) or `field` (`xt`, `yt`, `starts`, `field_steps`, `field_step`). `style` is `line`, `bar` or `stem`; also `bar_width`, `baseline`, `yerr` (list), `yerr_col`, `err_cap`, and `fill`: `{"curve": 2}` fills up to the second curve, `{"y": 0}` to a baseline, with `color` and `density`.
- The ranges fit all the curves, drawn in one frame. **Edit plot** shows the first curve of such a plot and keeps the others. Data given in the command is kept in the figure (up to 20,000 points, as comment lines), so a later `plot.update` with other ranges redraws it. The plot's own `depth`, `pen_color`, `thickness`, `line` and `style_val` apply where a curve sets none.
- Hostile or huge input is refused with an error: the expression length, the number of samples (20,000), the points of the curve and the size of a data file are limited.

## Settings and window

- `prefs.set`: sets a row of Global settings; the View toggles are commands too. `prefs.reset` sets rows back to the defaults (optionally with `names`, a list of resource names). `app.theme` and `app.ui_scale` set the theme and the UI scale.
- `app.messages` and `app.copy_messages` (with `levels`, for example `["warning","error"]`): the message history.
- `mode.favourite` and `mode.favourites`: the Favorites strip of mode buttons.
- `app.splash`: shows the start screen.
- `help.command_search`, `app.mouse_sheet`, `app.whats_new` and `app.keyboard_entry`: Command search, Mouse buttons, What's new and Enter point.
