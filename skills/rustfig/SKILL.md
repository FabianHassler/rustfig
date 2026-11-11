---
name: rustfig
description: Draw, edit and export xfig (.fig) figures with the RustFig MCP server (rustfig_execute, rustfig_render, rustfig_commands, rustfig_document). Use this whenever the user wants a .fig figure, an xfig or RustFig drawing, a schematic, sketch, block or energy diagram, or a plot for a paper or lecture notes; when they want to change a .fig file or make its labels LaTeX; or when they want a figure exported to PDF, EPS, SVG or PNG, even if they only say "draw" or "make a figure" and don't mention xfig or RustFig. Not for data dashboards or analysis plots the user wants as plotting code.
---

# RustFig figures

RustFig is an xfig replacement. Its MCP server, `rustfig-cli mcp`, holds one drawing and runs every RustFig command. Figures are saved as Fig 3.2 files (xfig and fig2dev read them) and exported to PDF, EPS, SVG, PNG, TikZ, PDF+LaTeX and more. Setup: https://fabianhassler.github.io/rustfig/manual/ai-assistants.html

## Tools

- `rustfig_commands {"filter": "shape."}`: commands and their parameters. Always pass a filter (the full list is about 50 KB). It comes from the running program, so trust it over this file.
- `rustfig_execute {"command": "shape.circle", "params": {"cx": 2400, "cy": 1800, "r": 600}}`: run one command (one undo step).
- `rustfig_render`: the drawing as PNG (`dpi`, default 100; `crop: true` shows the drawing's bounds instead of the page), with LaTeX typeset when TeX is installed and warnings as text.
- `rustfig_document`: the drawing as Fig text.

## Workflow

1. `file.new {}` or `file.open {"path": "/abs/path/figure.fig"}`. Always use absolute paths.
2. If labels use LaTeX packages or macros, set the preamble first: `doc.set_latex_preamble {"preamble": "\\usepackage{amsmath}"}`. A figure's texts get no other packages.
3. Draw with `shape.*`, `plot.add` and the other commands. Compute coordinates; don't estimate them.
4. Check with `object.list {}` (bounds of every object) and `rustfig_render`. Read the `warnings` of every reply.
5. **Set the bounding box: `doc.bounding_box_content {}`.** The exported page is the figure's bounding box, and its standard is the whole paper (Letter landscape): without this step a small drawing sits in a corner of an empty page, and anything off the paper is cut off. The box is saved in the .fig and stays fixed, so run it again whenever the drawing changes. The reply (and `doc.info` under `bounding_box`) gives the box as `x1, y1, x2, y2`. It is tight around the drawing; for a margin or a crop, set it yourself with `doc.set_bounding_box {"x1": …, "y1": …, "x2": …, "y2": …}` (Fig units).
6. `file.save {"path": "/abs/path/figure.fig"}`. Nothing reaches the disk before this.
7. `file.export {"path": "/abs/path/figure.pdf", "typeset_latex": true}`.

## Units

- 1200 Fig units per inch, y grows downwards, origin top left. 1 pt ≈ 16.7 units; a one-column figure is about 4000 units wide.
- `thickness` is a whole number, drawn as fig2dev draws it: 1 = 0.45 pt (hairline), then +0.9 pt per step (2 = 0.9 pt, 3 = 1.8 pt, 4 = 2.7 pt). Other widths can't be stored.
- Text `size` in points, `angle` in degrees counterclockwise.

## Shapes and style

`shape.polyline`, `shape.polygon` (`points: [[x,y], …]`), `shape.rectangle` (`x, y, width, height`, `radius`), `shape.circle` (`cx, cy, r`), `shape.text` (`x, y, text, size, justify, angle, special`; a `\n` in `text` writes one text per line, as Return does in Text mode; `anchor: "top"|"middle"|"baseline"` says what the point is on the text, `box: true` or `{margin, fill_color}` puts a filled box behind it), `shape.dimension_line` (`x1, y1, x2, y2`, `text` for an own label); ellipse, arc, spline, regular polygon and picture: see `rustfig_commands`.

- **Style** on every shape: `depth`, `thickness`, `pen_color`, `fill_color`, `area_fill`, `line` (solid, dashed, dotted, …), `style_val`, `cap`, `join`. Open shapes also take arrows: `forward_arrow`/`backward_arrow: true`, sized with `arrow_kind` (0-14 or a name: `stick`, `triangle`, `concave`, `convex`, `diamond`, `circle`, `dot`, `half_circle`, `square`, `reverse_triangle`, `half_filled_concave`, `half_triangle`, `half_concave`, `half_convex`, `wye`, `bar`, `fork`, `backward_fork`), `arrow_style` (hollow, filled), `arrow_width`, `arrow_height`, `mid_arrows: [0.5]` puts a head in the middle of a polyline, spline or arc (fractions of its length; the line becomes a compound of pieces), or one arrow at a time with `forward_arrow_spec: {"kind": 1, "style": "filled", "thickness": 15, "width": 60, "height": 120}`.
- **Colors:** `pen_color` and `fill_color` take a number (0 black, 1 blue, 2 green, 3 cyan, 4 red, 5 magenta, 6 yellow, 7 white, 8–31 shades), `"#rrggbb"`, `"#rgb"` or an X11 name (`"salmon"`): a color the figure has is reused, any other becomes a user color (up to 512). `color.add {"rgb": "#d9d9d9"}` returns the number.
- **Ready-made shapes** (braces, block arrows, zigzags, springs, coils, symbols): don't redraw them by hand; `library.search {"query": "brace"}` finds them in the bundled libraries and `library.place {"library": …, "name": …, "x": …, "y": …, "ops": ["rotate_cw", "larger"]}` places one (as a compound; `object.set_attributes`, `transform.*` then size and move it).
- **Fills:** `area_fill` 20 is the fill color itself, 0–19 darker, 21–39 lighter, 41–62 patterns, -1 none (for black: 0–20 runs white to black).
- **Depth:** lower is in front, default 50. Give fills and backgrounds a larger depth than the lines and labels on them.
- **Changing objects:** `object.set_attributes {"ids": [3, 4], …}` takes the same names plus `font_size`, `text` and more; also `transform.move`, `object.set_geometry`, `arrange.*`, `object.group`, `edit.delete`, `edit.duplicate`.

## LaTeX texts

`shape.text` with `special: true` is LaTeX, typeset in `rustfig_render`, in exports with `typeset_latex: true`, and by the document in LaTeX formats (PDF+LaTeX, TikZ):

```json
{"command": "shape.text", "params": {"x": 2400, "y": 600, "text": "$E_\\text{F}$", "size": 10, "justify": "center", "special": true}}
```

- Leave out `font` and `postscript`, so the text uses the document's LaTeX font (`\familydefault`; the preamble may change it).
- One backslash per command in the string, `\\` in JSON.
- A macro without its package gives `Undefined control sequence`: add the package to the preamble.
- Computer Modern stops at 24.88 pt; for larger texts add `\usepackage{lmodern}\usepackage[T1]{fontenc}` to the preamble.
- `x, y` is the baseline point at the justification. `rustfig_render {"latex": false}` is quicker for checking positions.

## Plots

```json
{"command": "plot.add", "params": {"x": 1200, "y": 600, "width": 3600, "height": 2400, "mode": "function", "expr": "sin(x)/x", "xmin": -10, "xmax": 10, "xlabel": "$x$", "ylabel": "$\\sin x / x$", "latex": true, "latex_numbers": true}}
```

- Modes: `function` (`expr` in `x`), `parametric` (`xt`, `yt`, `tmin`, `tmax`), `data` (`file` or `data`, `xcol`, `ycol`). Several curves: `"curves": [{"expr": "sin(x)", "pen_color": 4}, {"points": [[0, 1], [1, 2]], "marker": "circle", "line": "none"}]`.
- Axes: `"axes": "origin"` crosses them at (0, 0) with arrows and labels at the tips (with `"xticks": {"dir": "out"}`), `"none"` draws only the curves; `"grid": true` with `"grid_minor": true` and `"grid_style": "dashed"`.
- Tick numbers: `"xticks": {"scale": 3, "comma": true, "rotate": 45, "labels": [[1, "Mon"], [2, "Tue"]]}` (÷10³ with ×10³ at the axis end, decimal comma, turned numbers, own texts at given positions: bars and categories).
- A curve leaving `ymin`..`ymax` is cut at the window (poles too). Fill between curves in `curves`: `{"expr": "sin(x)", "fill": {"curve": 2, "color": 11, "density": 30}}, {"expr": "cos(x)"}`, or to the x axis with `"fill": {"y": 0}`.
- Bar charts: `"curves": [{"points": [[1, 3], [2, 5]], "style": "bar", "pen_color": 4}, {"points": [[1, 2], [2, 4]], "style": "bar", "pen_color": 2}]` (several bar curves stand side by side); `"style": "stem"` for stems; error bars with `"yerr": [0.3, 0.5]` or a data file column `"yerr_col": 3`. Category names on the x axis: `"xticks": {"labels": [[1, "A"], [2, "B"]]}`.
- More kinds (`mode` of a curve): `"polar"` (`"expr": "1+cos(t)"`), `"implicit"` (`"expr": "x^2+y^2=1"`, give the ranges), `"derivative"`/`"integral"` (`"of": 1`, an earlier curve), `"field"` (`"xt": "-y", "yt": "x", "starts": [[1, 0]]`).
- The reply's `id` is the plot compound; change it with `plot.update`.

## Exporting

- The format comes from the extension, or `format` (`file.export_formats {}` lists them). Options: `margin`, `magnification`, `dpi`, `background`, `embed`.
- For a LaTeX document: `format: "pdftex"` writes a PDF plus a `.pdf_t` with the texts, used as `\input{figure.pdf_t}`.
- Without MCP: `rustfig-cli convert in.fig out.pdf`.

## Pitfalls

- `file.open` and `file.new` replace the drawing without asking. Save first.
- Take ids from the replies; don't predict them: plots and other compounds use up ids for their members. Ids change when a file is reopened, so name objects you need again (`object.set_attributes {"ids": [id], "name": "legend"}`, then `"names": ["legend"]`).
- A parameter a command doesn't take is ignored with a warning; a bad one is an error that names it. Fix the call instead of retrying it.

## Example

```json
{"command": "file.new", "params": {}}
{"command": "shape.rectangle", "params": {"x": 600, "y": 600, "width": 4800, "height": 3000, "radius": 12, "pen_color": 1, "thickness": 2, "depth": 100}}
{"command": "shape.circle", "params": {"cx": 3000, "cy": 2100, "r": 600, "pen_color": 4, "fill_color": 4, "area_fill": 20, "depth": 90}}
{"command": "shape.text", "params": {"x": 3000, "y": 3300, "text": "$r = 0.5\\,\\mathrm{in}$", "size": 12, "justify": "center", "special": true}}
{"command": "doc.bounding_box_content", "params": {}}
{"command": "file.save", "params": {"path": "/abs/path/box.fig"}}
{"command": "file.export", "params": {"path": "/abs/path/box.pdf", "typeset_latex": true}}
```

Each line is one `rustfig_execute` call; call `rustfig_render` before saving to look at the result.
