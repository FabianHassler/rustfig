# MCP server

**Status:** 2026-11-11 (Q10: colors as text, arrow types by name, `shape.text` lines, `rustfig_render` `crop`; setup for Claude Code, Codex and others; the RustFig skill).

`rustfig-cli mcp` speaks the Model Context Protocol (JSON-RPC 2.0, one
message per line) on stdin/stdout. An AI assistant that runs it can draw,
edit, look at, save and export figures with RustFig's commands.

## Setup

1. **Install RustFig** (see the README: Homebrew, the Linux packages, Scoop
   or winget, or a release archive). `rustfig-cli --version` must work in a
   terminal. Clients started from a desktop launcher may not see the shell's
   `PATH`: give them the full path then (`which rustfig-cli`, for example
   `/opt/homebrew/bin/rustfig-cli`; on Windows `rustfig-cli.exe` in the
   install folder).
2. **Optionally install a TeX** (TeX Live, MacTeX, MiKTeX) for LaTeX labels:
   RustFig finds `pdflatex` or `lualatex` on its own.
3. **Add the server to the client:**
   - **Claude Code:** `claude mcp add --scope user rustfig -- rustfig-cli mcp`
     (`--scope user` for all projects; leave it out for the current one).
     `claude mcp list` shows it; `/mcp` in a session shows its state.
   - **OpenAI Codex CLI:** `codex mcp add rustfig -- rustfig-cli mcp`, or in
     `~/.codex/config.toml`:
     ```toml
     [mcp_servers.rustfig]
     command = "rustfig-cli"
     args = ["mcp"]
     ```
   - **Claude Desktop and other clients** with a JSON configuration
     (`claude_desktop_config.json`, Cursor's `mcp.json`, …):
     ```json
     {"mcpServers": {"rustfig": {"command": "/opt/homebrew/bin/rustfig-cli", "args": ["mcp"]}}}
     ```
4. **Install the skill** (recommended): [`skills/rustfig/SKILL.md`](../skills/rustfig/SKILL.md)
   tells the assistant how to use the tools: the workflow, coordinates,
   LaTeX texts, plots, exports and the usual mistakes. Copy the folder
   `skills/rustfig` to `~/.claude/skills/rustfig` (Claude Code, all
   projects), to `.claude/skills/rustfig` in a project, or to
   `~/.codex/skills/rustfig` (Codex). Clients without skills can be given
   the file's text as instructions.
5. **Restart the client** after updating RustFig: a running server keeps
   the old program.

Try it: "Draw a blue rounded box with a red circle in the middle and export
it as PDF." More examples, with the figures they gave, are in the manual's
[AI assistants](manual/ai-assistants.md) topic and in [`examples/ai`](../examples/ai).

**Online assistants** (claude.ai, ChatGPT in the browser) can't use RustFig
yet: they reach only remote MCP servers over HTTPS, and `rustfig-cli mcp`
runs on the user's computer over stdio.

## Tools

| Tool | Purpose |
|---|---|
| `rustfig_commands` | list commands and their parameters (`filter`: a substring of the id or label; the full list is about 50 KB) |
| `rustfig_execute` | run a command: `{"command": "shape.circle", "params": {"cx": 600, "cy": 600, "r": 300}}` |
| `rustfig_render` | the drawing as a PNG image (`dpi`, default 100; `latex`, default true: LaTeX texts typeset by TeX when one is installed, else shown as their source, with the page fitted to the source; TeX's errors, such as an undefined macro, come back as warnings next to the image; `crop`, default false: the drawing's content bounds plus a margin instead of the page, as View > Bounding box > Fit to drawing, without changing the figure's box) |
| `rustfig_document` | the drawing as Fig 3.2 text (or `format: "rustfig"`) |

Parameters match the UI (plan Q10): `pen_color` and `fill_color` take a color
number, `#rrggbb`, `#rgb` or an X11 name (the standard or user color with
that value is reused, else one is added in the same undo step; a clear error
when all 512 are used); `arrow_kind` takes 0-14 or a name (`stick`,
`triangle`, `concave`, `convex`, `diamond`, `circle`, `dot`, `half_circle`,
`square`, `reverse_triangle`, `half_filled_concave`, `half_triangle`,
`half_concave`, `half_convex`, `wye`, `bar`, `fork`, `backward_fork`; the
names that stand for a hollow or filled head also set the style, an explicit
`arrow_style` wins); `shape.text` with `\n` writes one text per line, spaced
as Return does in Text mode (separate texts, `{"ids": [...]}`); `shape.polyline`, `shape.spline` and `shape.arc` take `mid_arrows: [0.5]` (fractions of the line's length, ascending): an arrowhead centered at each position, the line cut into pieces glued into a compound (reply `{"id": compound}`). `shape.text` also takes `anchor` (`top`, `middle`, `baseline`: what the point `x, y` is on the text) and `box` (`true` or `{margin, fill_color, area_fill, pen_color, thickness}`: a filled box one depth behind the text, glued into a compound). `object.annotate_angle` draws a measured angle (an arc with its label, or `mark: "right"`); `shape.dimension_line` takes `label_along`.

Coordinates are Fig units: 1200 per inch, y grows downwards. Open and save
files with `file.open` / `file.save` through `rustfig_execute`.

## Plots

`plot.add` draws a plot as one compound and `plot.update` redraws it with
changed parameters (`plot.get` returns them; `rustfig_commands` with the
filter `plot` lists every parameter). Everything the Plot dialog offers is a
parameter, and a plot's parameters are kept in the figure, so an edited
plot redraws the same. Axes and grid (the plot's `axes`, `axis_arrows`,
`grid`, `grid_minor` and `grid_style`):

- `axes`: `"frame"` (ticks and numbers on the edges of the box, the default),
  `"origin"` (axes crossing at (0, 0) or the nearest edge, arrowheads at the
  tips, labels at the tips, ticks and numbers on one side) or `"none"`.
  `axis_arrows: false` leaves the arrowheads off.
- `grid: true` with `grid_minor: true` (lines at the minor ticks too) and
  `grid_style`: `"dotted"` (default), `"dashed"`, `"solid"`.

Tick label format (`xticks` and `yticks` objects, as in the dialog):

- `scale: 3` divides the numbers by 10^3 and writes `x10^3` at the axis end
  (`$\times10^{3}$` with `latex_numbers`); `comma: true` is the decimal comma
  (`0{,}5` in LaTeX numbers); `rotate: 45` turns the numbers by degrees,
  counterclockwise.
- `labels: [[1, "Mon"], [2, "Tue"]]` puts the ticks at these values (inside
  the range) with these texts, so categories work: positions 1..n, the texts
  are used as typed (LaTeX when `latex_numbers`). `numbers: false` hides the
  texts, `ticks: false` the marks.

Curves cut and filled: a curve leaving `ymin`..`ymax` (or the x range) is cut
at the window, its pieces ending on the frame's edge, and a curve in `curves`
takes `fill`: `{"curve": 2}` (the area between it and curve 2, numbered from
1), `{"y": 0}` (between it and the line y = 0; the default when `fill` is
`{}`), with `color` (a color number, default the curve's pen color) and
`density` (xfig's area fill, 20 full color). The area is a polygon without
outline, drawn behind the grid and the curves (part `fill`).

Bars, stems and error bars, per curve in `curves`: `style` is `"line"`
(default), `"bar"` or `"stem"`; bars rise from `baseline` (default 0) with
`bar_width` (fraction of the x spacing, 0.8) and, in several bar curves, stand
side by side (grouped); give the curves the same x values. `yerr` is the list
of error values of `points` (one value applies to all points; kept in the
figure) and `yerr_col` the column of a data `file` or `data` text; `err_cap` is
the cap width in Fig units. The reply of `plot.add` and `plot.update` counts
`bars` (bars, stems and error bar lines). Data from a file needs no list:
`{"file": "d.csv", "xcol": 1, "ycol": 2, "yerr_col": 3, "style": "bar"}`.

More curve kinds, as the `mode` of a plot or of a curve in `curves`:
`"polar"` (`expr` is r in `t` or `theta`, `tmin`/`tmax`, default 0..2pi),
`"implicit"` (`expr` is `f(x,y)` or an equation `"x^2+y^2=1"`; marching
squares on `samples` cells per axis, at most 300, over `xmin`..`ymax`, default
-10..10), `"derivative"` and `"integral"` (`of`: the number, from 1, of an
earlier curve; integral from 0 at its first point) and `"field"` (`xt`, `yt`
the components P and Q of the field in `x`, `y`; `starts: [[x, y]...]`, at most
60; `field_steps` each way, default 400, at most 5000; `field_step` the step
length, default 1/100 of the window's diagonal; RK4; linear axes only).
Sizes and work are capped; hostile expressions end in an error or a plot.

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First version. |
| 2026-11-06 | Setup for Claude Code, Codex and JSON-configured clients; the RustFig skill (`skills/rustfig/SKILL.md`). |
| 2026-11-11 | Plots: axes crossing at the origin, no axes, grid with minor lines and a line style (Q1). |
| 2026-11-11 | Plots: tick labels with a scale factor, decimal comma, own labels and rotation (Q2). |
| 2026-11-11 | Plots: filled areas between curves or to a baseline (Q3). |
| 2026-11-11 | Plots: bars (grouped), stems and error bars (Q4). |
| 2026-11-11 | Plots: polar, implicit, derivative, integral and field-line curves (Q5). |
| 2026-11-11 | Q10: `pen_color`/`fill_color` as `#rrggbb` or X11 name, `arrow_kind` by name, `shape.text` with `\n` one text per line, `rustfig_render` `crop`. |
| 2026-11-11 | Q7: `mid_arrows` on shape.polyline, spline and arc. |
| 2026-11-11 | Q8: `shape.text` `box` and `anchor`. |
| 2026-11-11 | Q9: `object.annotate_angle`, `shape.dimension_line` `label_along`. |
