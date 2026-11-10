---
name: rustfig
description: Draw, edit and export xfig (.fig) figures with the RustFig MCP server (rustfig_execute, rustfig_render, rustfig_commands, rustfig_document). Use this whenever the user wants a .fig figure, an xfig or RustFig drawing, a schematic, sketch, block or energy diagram, or a plot for a paper or lecture notes; when they want to change a .fig file or make its labels LaTeX; or when they want a figure exported to PDF, EPS, SVG or PNG, even if they only say "draw" or "make a figure" and don't mention xfig or RustFig. Not for data dashboards or analysis plots the user wants as plotting code.
---

# RustFig figures

RustFig is an xfig replacement. Its MCP server, `rustfig-cli mcp`, holds one drawing and exposes every RustFig command, the same commands its menus run. Figures are saved as Fig 3.2 files, which xfig and fig2dev read too, and exported with RustFig's built-in fig2dev (PDF, EPS, SVG, PNG, TikZ, PDF+LaTeX and more).

Setup (installing RustFig and adding the server to Claude Code, Claude Desktop, Codex or Cursor) is in the manual's "AI assistants" topic: https://fabianhassler.github.io/rustfig/manual/ai-assistants.html

## Tools

| Tool               | Use                                                                                                                                                                                                                                         |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rustfig_commands` | List commands with their parameters. Always pass a `filter` (`"shape."`, `"text"`, `"plot."`, `"object.set"`, `"arrange."`, `"file."`): the full list is about 50 KB. It is generated from the running program, so trust it over this file. |
| `rustfig_execute`  | Run one command: `{"command": "shape.circle", "params": {"cx": 2400, "cy": 1800, "r": 600}}`. Each mutating command is one undo step (`edit.undo`, `edit.redo`).                                                                            |
| `rustfig_render`   | The drawing as a PNG image (`dpi`, default 100). LaTeX texts are typeset by TeX when one is installed. Warnings (for example `Undefined control sequence`) come back as text next to the image.                                             |
| `rustfig_document` | The drawing as Fig 3.2 text (`format: "rustfig"` for RustFig's JSON).                                                                                                                                                                       |

## Workflow

1. **Start clean or open a file.** `file.new {}` or `file.open {"path": "/abs/path/figure.fig"}`. Use absolute paths: the server's working folder is the client's.
2. **Set the document first.** Units and paper: `doc.set_units {"units": "metric"}`, `doc.set_paper {"paper": "A4"}`. When labels use LaTeX packages or macros, set the preamble before drawing them: `view.set_latex_preamble {"preamble": "\\usepackage{bm}"}`.
3. **Draw** with `shape.*`, `plot.add` and the other commands. Compute coordinates instead of estimating them, especially for symmetric shapes and curves.
4. **Check** with `object.list {}` (id, type, depth and bounds of every object; with TeX installed, LaTeX texts measure as typeset) and `rustfig_render`. Bounds catch shapes that are off-center or too large, which a small preview hides. Read the warnings next to the image.
5. **Save**: `file.save {"path": "/abs/path/figure.fig"}`. Nothing reaches the disk before this.
6. **Export**: `file.export {"path": "/abs/path/figure.pdf", "typeset_latex": true}`. Check the `warnings` in the reply.

## Coordinates and units

- Fig units: 1200 per inch, y grows downwards, the origin is the top-left corner. Use positive coordinates.
- Lengths in points: 1 pt = 1200/72 ≈ 16.7 units. A one-column figure in a paper is about 3.3 in (4000 units) wide.
- Line `thickness` is in 1/80 inch: 1 is a thin line, 2 a normal one.
- Text `size` is in points. `angle` is in degrees, counterclockwise.
- `units` (inches or metric) changes only the rulers, grid and the file header, not the coordinates.

## Shapes and style

| Command                                                                                                        | Parameters                                                                                      |
| -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `shape.polyline`                                                                                               | `points: [[x,y], …]` (a line or open path; arrows with `forward_arrow`, `backward_arrow`)       |
| `shape.polygon`                                                                                                | `points` (closed)                                                                               |
| `shape.rectangle`                                                                                              | `x, y, width, height`, `radius` for rounded corners (1/80 inch)                                 |
| `shape.circle`                                                                                                 | `cx, cy, r`                                                                                     |
| `shape.ellipse`, `shape.arc`, `shape.spline`, `shape.regular_polygon`, `shape.dimension_line`, `shape.picture` | see `rustfig_commands`                                                                          |
| `shape.text`                                                                                                   | `x, y, text, size, justify (left/center/right), angle, special, font, postscript, color, depth` |

Every shape takes the style parameters `depth`, `thickness`, `pen_color`, `fill_color`, `area_fill`, `line` (solid, dashed, dotted, dash-dotted, …), `style_val`, `forward_arrow`, `backward_arrow`.

- **Colors** are numbers. The standard ones are 0 black, 1 blue, 2 green, 3 cyan, 4 red, 5 magenta, 6 yellow, 7 white, and 8–31 shades. Other colors: `color.new {"rgb": "#d9d9d9"}` returns `{"color": 32}`, which you then use as a number.
- **Fills:** `area_fill` 20 is the fill color at full saturation, 0–19 darker shades toward black, 21–39 lighter tints toward white, 41–62 patterns, -1 no fill. For black or the default color, 0–20 runs from white to black instead. For a light gray, define the gray with `color.new` and fill at 20.
- **Depth:** 0–999, lower is in front. Each new object gets depth 50 unless you pass `depth`. Give backgrounds and fills a larger depth than the lines and labels on them.
- **Changing objects:** `object.set_attributes {"ids": [3, 4], …}` takes the same style names, plus `font_size`, `text`, `special` and others. `object.set_geometry`, `arrange.set_depth`, `arrange.align`, `object.group`, `edit.delete` and `edit.duplicate` work on ids too.

## Text and LaTeX

A text with the **TeX flag** (`special: true`) is LaTeX source. It is typeset by TeX in `rustfig_render`, in PDF, EPS, SVG and PNG exports with `typeset_latex: true`, and by the document's LaTeX in the LaTeX export languages (PDF+LaTeX, TikZ, pict2e).

```json
{
  "command": "shape.text",
  "params": {
    "x": 2400,
    "y": 600,
    "text": "$E_\\text{F}$",
    "size": 10,
    "justify": "center",
    "special": true
  }
}
```

- Pass `special: true` and leave out `postscript` and `font`, so the text is set in the document's own LaTeX font. With `postscript: true` fig2dev sets it in a PostScript font (Times for font 0).
- Write one backslash per LaTeX command in the actual string, which is `\\` inside JSON. The saved .fig file doubles it again (`$\\alpha$`), which is normal.
- Macros from packages or the user's own macros need `view.set_latex_preamble`. Otherwise TeX reports `Undefined control sequence`, the text shows as its source, and the warning names the text.
- The `x, y` of a text is its baseline point at the justification (left end, center or right end).
- `rustfig_render` with `latex: false` shows the source instead of typesetting it, which is quick for checking positions. For small overlaps, render at `dpi` 200–300 (up to 600).

## Plots

`plot.add` draws a function, a parametric curve or data as one compound with a frame, ticks and numbers:

```json
{
  "command": "plot.add",
  "params": {
    "x": 1200,
    "y": 600,
    "width": 3600,
    "height": 2400,
    "mode": "function",
    "expr": "sin(x)/x",
    "xmin": -10,
    "xmax": 10,
    "xlabel": "$x$",
    "ylabel": "$\\sin x / x$",
    "latex": true,
    "latex_numbers": true
  }
}
```

- `latex: true` makes the axis labels TeX text. `latex_numbers: true` writes the tick numbers in math mode too, so negative numbers get a real minus sign (`$-1.5$`), not a hyphen.
- Modes: `"function"` (`expr` in `x`), `"parametric"` (`xt`, `yt` in `t`, with `tmin`, `tmax`), `"data"` (`file` or the text in `data`, with columns `xcol`, `ycol`). Also `log_x`, `log_y`, `grid`, `xticks`, `yticks`.
- Several curves in one plot: `"curves": [{"expr": "sin(x)", "pen_color": 4}, {"points": [[0, 1], [1, 2]], "marker": "circle", "line": "none"}]`. Each curve takes `pen_color`, `thickness`, `line`, `marker` (circle, square, triangle, diamond, cross, plus), `marker_size`, `marker_fill`; the plot takes `depth` and a default style.
- An empty `expr` with all four ranges draws only the frame, ticks and labels (axes for curves added later).
- The reply's `id` is the compound. `plot.edit` and `plot.update` change it later. Annotations are separate objects on top.

## Exporting

- `file.export {"path": "out.pdf", "typeset_latex": true}`. The format comes from the extension, or pass `format` (`file.export_formats {}` lists the formats and their options). `typeset_latex` defaults to false, which writes LaTeX texts as their raw source.
- Useful options: `margin` (Fig units around the drawing), `magnification` (percent), `dpi` and `background` for PNG, `embed: true` (PDF, SVG, PNG: the Fig text goes into the file, so RustFig opens it again as a drawing).
- For a LaTeX document, `format: "pdftex"` writes a PDF with the graphics and a `.pdf_t` file with the texts, which the document's LaTeX typesets in its own fonts: `\input{figure.pdf_t}`.
- The page fits the drawing: the bounds of all objects, including the typeset texts.
- Without MCP the command line converts too: `rustfig-cli convert in.fig out.pdf`, or `fig2dev -L pdf in.fig out.pdf` (fig2dev's own options).

## Pitfalls

- **One drawing per server:** `file.open` and `file.new` replace the current drawing without asking. Save first.
- **Moving** objects without changing them: `transform.move {"ids": [...], "dx": 150, "dy": 0}` (also compounds). `object.set_geometry` repositions and rescales.
- **Ids** are numbers returned by the drawing commands and by `object.list`. Grouping (`object.group`) makes a new compound id.
- **Name objects you will need again:** ids change when a file is reopened. `object.set_attributes {"ids":[id],"name":"legend"}` stores the name in the .fig; afterwards `"names": ["legend"]` works wherever `"ids"` does (a name selects every top-level object carrying it).
- **Read the reply's `warnings`:** a parameter the command does not take is ignored, and the warning names it. Colors are `pen_color` and `fill_color` everywhere (`shape.text` also takes `color`).
- **Hostile input is rejected, not guessed:** an error names the parameter. Read it and fix the call instead of retrying the same one.

## Example

"Draw a blue rounded box with a red circle in the middle and label the radius."

```json
{"command": "file.new", "params": {}}
{"command": "shape.rectangle", "params": {"x": 600, "y": 600, "width": 4800, "height": 3000, "radius": 12, "pen_color": 1, "thickness": 2, "depth": 100}}
{"command": "shape.circle", "params": {"cx": 3000, "cy": 2100, "r": 600, "pen_color": 4, "fill_color": 4, "area_fill": 20, "depth": 90}}
{"command": "shape.text", "params": {"x": 3000, "y": 3300, "text": "$r = 0.5\\,\\mathrm{in}$", "size": 12, "justify": "center", "special": true}}
{"command": "file.save", "params": {"path": "/abs/path/box.fig"}}
{"command": "file.export", "params": {"path": "/abs/path/box.pdf", "typeset_latex": true}}
```

Each line is one `rustfig_execute` call. Call `rustfig_render` after the drawing commands to look at the result before saving.
