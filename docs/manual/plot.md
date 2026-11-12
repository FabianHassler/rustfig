# Plotting a curve

RustFig can draw a graph for you: a function y = f(x), a parametric curve x(t), y(t), or the columns of a data file. The result is ordinary Fig objects, so it prints, exports and scales like the rest of the drawing and opens in xfig. xfig has no such mode; this is an addition.

A plot is **one compound**: the frame, the ticks with their numbers, an optional grid, the axis labels and the curve as polylines. Open the compound (see [Compounds](compounds.md)) to touch up one part. The plot's settings are kept in the compound's comment (a line starting `rustfig-plot:`), which Fig 3.2 files keep, so a plot can be edited again later, also after saving and opening.

## Plot

Key `y`. The last button of the Drawing column, beside the library button; the same as the Tools menu's **Plot curve...** (Ctrl+Shift+P, ⇧⌘ P on macOS).

- Left, “first corner”: click where one corner of the plot's frame goes, as in Box mode. A rubber-band rectangle follows the pointer and the left button now says “final point”: click the opposite corner and the Plot dialog opens for that frame. A single click opens nothing. Nothing is drawn until you press **Draw plot** in the dialog; Cancel leaves the drawing as it was.
- The right button, “cancel”, drops the frame you have started. The middle button does nothing in this mode.

Tools > **Plot curve...** opens the same dialog without placing a frame: the plot goes to the origin at the default size.

## The Plot dialog

Choose what to draw:

- **y = f(x)**: type the expression and the range of x. Samples (default 400) are spread evenly and more are added where the curve bends, so peaks stay sharp.
- **x(t), y(t)**: two expressions in t and the range of t (default 0 to 2π, written `2*pi`).
- **r(t) polar**: the radius as an expression in `t` (or `theta`) and the range of t (default 0 to 2π): `1+cos(t)` is a cardioid.
- **f(x,y) = 0**: an implicit curve, an equation such as `x^2 + y^2 = 1` (or a formula, which is compared with 0), found by marching squares on a grid of up to 300 by 300 cells over the x and y ranges (default -10 to 10 when empty; a coarser grid at the default 400 samples is capped at 300). Branches are chained into polylines.
- **Field lines**: the components P(x,y) and Q(x,y) of a vector field and **start points** (`x, y; x, y`, up to 60). Each line is followed forward and backward from its start by Runge-Kutta (RK4) in steps of a fixed length (default 1/100 of the window's diagonal) up to the given number of steps each way, until it leaves the window or the field vanishes. Linear axes only; the total work is capped.
- **Data file**: a CSV or whitespace separated file (comma, semicolon, tab or blanks). Give the number of the x and y columns, counting from 1; x column 0 uses the row number. Lines starting with # or % and lines that are not numbers (a header) are skipped. Files up to 16 MB are read. This works on the desktop only; on the web use the command `plot.add` with the text as `data`.

**Curve options** (collapsible, under Function) apply to the curve:

- **Also draw**: the *derivative* (dashed) and the *integral* (dotted) of the first curve, as further curves of the plot: the derivative by central differences over the curve's points, the integral by trapezoids from 0 at its first point. With the commands, a curve `{"mode": "derivative", "of": 1}` is of an earlier curve (any, also another derivative).
- **Draw as**: *line* (the default), *bars* or *stems*. A bar is a filled box from the baseline (**from y =**, default 0) to each data point, **width** a fraction (default 0.8) of the smallest distance between the x values; the box has the curve's pen color as outline and fill (Fill color and density below change the fill; line *none* draws it without outline). When several curves are bars they stand side by side in each slot (grouped bars), in the order of the curves, so give them the same x values. A stem is a vertical line from the baseline to the point (add a marker with the commands for a Dirac plot). The ranges fit the baseline and half a bar at each end; bars and stems leaving the window are cut at its edge.
- **Error bars**: for a data file, the **column** of the error values: each point gets a vertical line from y - e to y + e with a cap (**cap width**, Fig units) at each end that is inside the window. With the commands the values are a list `yerr` (one for each point, or one for all) next to `points`, or `data` with `yerr_col`; such values are kept in the figure (comment lines `rustfig-plot-yerr:`), as the points are. Error bars belong to data curves.
- **Fill**: a filled area behind the curve, as a polygon behind the grid and the curves. *To the line y =* fills between the curve and a horizontal line (the x axis for y = 0; a line outside the window sits on its edge); where the curve is cut at the window, each piece gets its own polygon. *Up to curve* fills between this curve and another curve of the plot (the number counts from 1; **Add curve y = f(x)** under Curve options adds a function curve to the plot, a row for each, with Remove): the area between sin and cos, for instance. Crossing curves give two areas. **Fill color** is a color number (empty: the curve's color) and **density** the xfig area fill: 20 the full color, below it darker, above it lighter, 40 white. The area has no outline.

The ranges of x and y can be left empty to fit the curve (the y range of a curve with poles ignores a few extreme values, so 1/x does not flatten everything else); the numbers are rounded to tick values. Numbers may be expressions such as `2*pi`.

The dialog has four groups: **Function** (the mode, its fields, samples), **Range** (x and y, each with **Logarithmic**: powers of ten, the range must be positive), **Axes** and **Labels**.

**Axes** has a small table with the rows **x axis** and **y axis** and the columns **Ticks** and **Numbers**: each box turns the tick marks or the numbers of that axis on or off by itself. **Axes** chooses where the axes stand:

- **On the frame** (the default): ticks and numbers on the edges of the frame; **Box** draws the frame itself. With Box off the ticks still stand on the edges of the (invisible) frame.
- **Crossing at the origin**: two axes cross at (0, 0), or at the nearest edge of the range when 0 lies outside it (the lower edge of a logarithmic axis). They have an arrowhead at the tip (**Arrows**), the x label stands right of the x axis's tip and the y label above the y axis's tip, and the ticks and numbers are on one side of each axis (numbers below the x axis and left of the y axis; the number at the crossing is the x axis's alone). Set the tick direction of each axis to *out* for the usual look. There is no box.
- **None**: no box, axes, ticks or numbers, only the curves (and the grid and labels).

**Grid** draws lines at the numbered ticks, dotted by default; **Minor lines** adds lines at the minor ticks (a linear axis without minor ticks set gets 4 between two majors; the lines are dotted at twice the gap) and **Lines** chooses the style of the major ones: dotted, dashed or solid. Plots made before these switches (one "axes" setting) open with Box, ticks and numbers all on, or all off if axes were off.

**Labels** holds the x and y label, plain text or LaTeX text with the TeX flag when **LaTeX labels** is on (they are then typeset when you export to LaTeX or turn on View > Typeset LaTeX text). **LaTeX numbers** writes the tick numbers as `$...$` texts with the TeX flag too (a power of ten as `$10^{-5}$`, multiples of pi as `$\pi$`). LaTeX labels and numbers are texts in the LaTeX font with the TeX flag set: the canvas typesets them when TeX is installed (View > Typeset LaTeX text), and the LaTeX, pstex and pdftex exports pass them on unchanged.

**x axis tick settings** and **y axis tick settings** (collapsible, under Axes) set the details of each axis:

- **Manual**: the first tick, the step (or a count of intervals); empty fields stay automatic. Without Manual the ticks are chosen for you, as in plots made earlier.
- **Minor per major** (0 to 9) and the **length** of the ticks (Fig units, 1200 per inch; minor ticks are half as long), their **direction** (in, out or both).
- **Number format**: automatic, a fixed number of **decimals**, **scientific**, or **multiples of pi** (0, pi/2, pi, 3pi/2; the automatic step is then pi/2).
- **Scale factor** (a power of ten, e.g. 3): the numbers are divided by 10^3 (2000 shows as 2) and `×10^3` stands at the axis's end (right of the x axis, above the y axis; as `$\times10^{3}$` with LaTeX numbers). Not on a logarithmic axis.
- **Decimal comma**: 0,5 for 0.5 (in LaTeX numbers `0{,}5`, so TeX does not add a space).
- **Rotate numbers by** an angle in degrees, counterclockwise (45: the numbers rise to the right and end under their ticks; -45 start under them; 90 reads upward).
- **Own labels**: one line per label, the position, a space and the text (`1 Mon`, `2 Tue`; the position may be an expression such as `pi/2`). The ticks stand exactly at these positions inside the range (also categories: positions 1, 2, 3...) and carry these texts instead of numbers; **Numbers** off hides the texts, **Ticks** off the marks. With LaTeX numbers the text is used as typed (write `$\alpha$` yourself) and made a TeX text; on a logarithmic axis the position is the value, not its logarithm.
- On a logarithmic axis the step counts decades and the minor ticks stand at 2 to n+1 times a decade.

Too many ticks, a step of zero or less, and similar values are refused with an error. The settings are kept in the plot's comment like the rest.

The line style, thickness, color, depth and the font of the numbers and labels are the current ones in the attribute panel; set them before you plot.

Expressions use `+ - * / ^` (also × ÷ and `**`), parentheses, the constants `pi` and `e`, and the functions sin, cos, tan, asin, acos, atan, sinh, cosh, tanh, exp, ln (also log: natural), log10, log2, sqrt, abs, floor, ceil, sign, heaviside (0, 0.5 at 0, 1), the special functions gamma, lgamma (ln |Γ|), erf, erfc and the Bessel functions j0, j1, y0, y1 (ordinary: J, Y) and i0, i1, k0, k1 (modified: I, K), and with two arguments atan2(y, x), min(a, b), max(a, b), pow(a, b) and the Bessel functions of integer order jn(n, x), yn(n, x), in(n, x), kn(n, x) (n is rounded; |n| up to 1000). y, k and their orders are not defined for x < 0 (the curve stops), gamma not at 0 and the negative integers. `-x^2` means -(x^2) and `2^3^2` is 2^(3^2). An expression of up to 500 characters and nested up to 64 deep is accepted; an error names the position.

An empty expression draws only the frame, ticks, numbers and labels; give both ranges then (y has no curve to fit). A curve that lies wholly outside the ranges also leaves just the frame, with a warning in the message panel; wider ranges later show it.

Where the function is not defined (division by zero, the root of a negative number) the curve simply stops: there is never a line across a pole such as that of 1/x or tan(x). Where it leaves the y range (or the x range) it is cut at the window: the part outside is dropped and the line ends exactly on the frame's edge, so tan(x) between -3 and 3 shows each branch running up to the top and down from the bottom of the frame.

## Edit plot

A plot stays one compound that carries the `rustfig-plot:` comment, and every part RustFig generated (the frame, grid lines, ticks, numbers, labels and the curve) carries a marker comment such as `rustfig-plot-part: curve`.

- **Edit inside**: use Open compound (see [Compounds](compounds.md)) to move, restyle or delete single parts, or to add your own objects (an arrow, a note, a second curve you drew by hand).
- **Redraw**: while the compound keeps its plot comment, Tools > **Edit plot...** (and Edit mode's left button on the plot) opens the dialog filled with the plot's settings, and **Draw plot** redraws it in its present frame, where you have moved or scaled it. Redraw replaces only the marked parts. Objects you added are kept, with their changes. A generated part you edited keeps its marker, so it is replaced by the redraw as well; to keep such a part, remove its marker comment first (Edit mode, comments) or draw a copy. Undo brings the old plot back.
- **Break**: Break compound (see [Compounds](compounds.md)) turns the plot into an ordinary drawing: the plot comment goes away with the compound and the parts lose their markers. It can no longer be edited as a plot.

## Commands

**Several curves** (scripts and MCP for now): `plot.add` takes `curves`, a list in which each curve has its own source (`expr`, `xt`/`yt`, a data `file`, `data` text or `points`) and style (`pen_color`, `thickness`, `line`, `style_val`; `line: "none"` draws markers only) and markers (`marker`: circle, square, triangle, diamond, cross or plus; `marker_size` in Fig units; `marker_fill`, `mode` also `polar` (`expr` in `t`), `implicit` (`expr`, an equation), `derivative`/`integral` (`of`), `field` (`xt`, `yt`, `starts`, `field_steps`, `field_step`), `style` (`line`, `bar`, `stem`), `bar_width`, `baseline`, `yerr` (list), `yerr_col`, `err_cap`, `fill`: `{"curve": 2}` fills up to the second curve, `{"y": 0}` to a baseline, with `color` and `density`). The ranges fit all the curves. Data given in the command is kept in the figure (up to 20,000 points, as comment lines), so a later `plot.update` with other ranges redraws it. The plot's own `depth`, `pen_color`, `thickness`, `line` and `style_val` apply where a curve sets none. **Edit plot** shows the first curve of such a plot and keeps the others.

For scripts and MCP (see [Automation](automation.md)): `plot.add` draws a plot (frame `x`, `y`, `width`, `height`; the settings `mode`, `expr`, `xt`, `yt`, `tmin`, `tmax`, `file`, `xcol`, `ycol`, `data`, `xmin`, `xmax`, `ymin`, `ymax`, `samples`, `frame`, `xticks`, `yticks`, `axes` (`"frame"`, `"origin"` or `"none"`; the old `false` turns box, ticks and numbers off), `axis_arrows`, `grid`, `grid_minor`, `grid_style` (`dotted`, `dashed`, `solid`), in `xticks` and `yticks` also `scale`, `comma`, `labels` (`[[1, "Mon"], [2, "Tue"]]`) and `rotate`, `log_x`, `log_y`, `xlabel`, `ylabel`, `latex` (the axis labels as LaTeX), `latex_numbers` (the tick numbers as LaTeX math, `$-1.5$`)), `plot.update` changes some settings of an existing plot, `plot.get` returns them, `plot.panel` and `plot.edit` open the dialog. Hostile or huge input is refused with an error: the expression length, the number of samples (20,000), the points of the curve and the size of a data file are limited.
