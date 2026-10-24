# Plotting a curve

RustFig can draw a graph for you: a function y = f(x), a parametric curve x(t), y(t), or the columns of a data file. The result is ordinary Fig objects, so it prints, exports and scales like the rest of the drawing and opens in xfig. xfig has no such mode; this is an addition.

A plot is **one compound**: the frame, the ticks with their numbers, an optional grid, the axis labels and the curve as polylines. Open the compound (see [Compounds](compounds.md)) to touch up one part. The plot's settings are kept in the compound's comment (a line starting `rustfig-plot:`), which Fig 3.2 files keep, so a plot can be edited again later, also after saving and opening.

## Plot

Key `y`. The last button of the Drawing column, beside the library button; the same as the Edit menu's **Plot curve...** (Ctrl+Shift+P, Cmd+Shift+P on macOS).

- Left, “first corner”: click where one corner of the plot's frame goes, as in Box mode. A rubber-band rectangle follows the pointer and the left button now says “final point”: click the opposite corner and the Plot dialog opens for that frame. A single click opens nothing. Nothing is drawn until you press **Draw plot** in the dialog; Cancel leaves the drawing as it was.
- The right button, “cancel”, drops the frame you have started. The middle button does nothing in this mode.

Edit > **Plot curve...** opens the same dialog without placing a frame: the plot goes to the origin at the default size.

## The Plot dialog

Choose what to draw:

- **y = f(x)**: type the expression and the range of x. Samples (default 400) are spread evenly and more are added where the curve bends, so peaks stay sharp.
- **x(t), y(t)**: two expressions in t and the range of t (default 0 to 2π, written `2*pi`).
- **Data file**: a CSV or whitespace separated file (comma, semicolon, tab or blanks). Give the number of the x and y columns, counting from 1; x column 0 uses the row number. Lines starting with # or % and lines that are not numbers (a header) are skipped. Files up to 16 MB are read. This works on the desktop only; on the web use the command `plot.add` with the text as `data`.

The ranges of x and y can be left empty to fit the curve (the y range of a curve with poles ignores a few extreme values, so 1/x does not flatten everything else); the numbers are rounded to tick values. Numbers may be expressions such as `2*pi`.

**Axes with ticks and numbers** draws the frame, ticks and numbers; **Grid** adds dotted lines at the numbered ticks; **Logarithmic x** and **y** put the axis in powers of ten (the range must be positive). The x and y labels are plain text, or LaTeX special text when **Labels are LaTeX** is on (they are then typeset when you export to LaTeX or turn on View > Typeset LaTeX text). **Numbers as LaTeX** writes the tick numbers as `$...$` special texts too (a power of ten as `$10^{-5}$`, multiples of pi as `$\pi$`). LaTeX labels and numbers are special texts with the LaTeX font, as xfig's text with the Special flag: the canvas typesets them when TeX is installed (View > Typeset LaTeX text), and the LaTeX, pstex and pdftex exports pass them on unchanged.

**x ticks** and **y ticks** (collapsible sections of the dialog) set the ticks of each axis:

- **Manual**: the first tick, the step (or a count of intervals); empty fields stay automatic. Without Manual the ticks are chosen for you, as in plots made earlier.
- **Minor per major** (0 to 9) and the **length** of the ticks (Fig units, 1200 per inch; minor ticks are half as long), their **direction** (in, out or both).
- **Numbers**: shown or not; automatic, a fixed number of **decimals**, **scientific**, or **multiples of pi** (0, pi/2, pi, 3pi/2; the automatic step is then pi/2).
- On a logarithmic axis the step counts decades and the minor ticks stand at 2 to n+1 times a decade.

Too many ticks, a step of zero or less, and similar values are refused with an error. The settings are kept in the plot's comment like the rest.

The line style, thickness, colour, depth and the font of the numbers and labels are the current ones in the attribute panel; set them before you plot.

Expressions use `+ - * / ^` (also × ÷ and `**`), parentheses, the constants `pi` and `e`, and the functions sin, cos, tan, asin, acos, atan, sinh, cosh, tanh, exp, ln (also log: natural), log10, log2, sqrt, abs, floor, ceil, sign, and with two arguments atan2(y, x), min(a, b), max(a, b), pow(a, b). `-x^2` means -(x^2) and `2^3^2` is 2^(3^2). An expression of up to 500 characters and nested up to 64 deep is accepted; an error names the position.

Where the function is not defined (division by zero, the root of a negative number) or beyond the plot's range, the curve simply stops: there is never a line across a pole such as that of 1/x or tan(x), and parts outside the frame are cut at the frame.

## Edit plot

A plot stays one compound that carries the `rustfig-plot:` comment, and every part RustFig generated (the frame, grid lines, ticks, numbers, labels and the curve) carries a marker comment such as `rustfig-plot-part: curve`.

- **Edit inside**: use Open compound (see [Compounds](compounds.md)) to move, restyle or delete single parts, or to add your own objects (an arrow, a note, a second curve you drew by hand).
- **Redraw**: while the compound keeps its plot comment, Edit > **Edit plot...** (and Edit mode's left button on the plot) opens the dialog filled with the plot's settings, and **Draw plot** redraws it in its present frame, where you have moved or scaled it. Redraw replaces only the marked parts. Objects you added are kept, with their changes. A generated part you edited keeps its marker, so it is replaced by the redraw as well; to keep such a part, remove its marker comment first (Edit mode, comments) or draw a copy. Undo brings the old plot back.
- **Break**: Break compound (see [Compounds](compounds.md)) turns the plot into an ordinary drawing: the plot comment goes away with the compound and the parts lose their markers. It can no longer be edited as a plot.

## Commands

For scripts and MCP (see [Automation](automation.md)): `plot.add` draws a plot (frame `x`, `y`, `width`, `height`; the settings `mode`, `expr`, `xt`, `yt`, `tmin`, `tmax`, `file`, `xcol`, `ycol`, `data`, `xmin`, `xmax`, `ymin`, `ymax`, `samples`, `axes`, `grid`, `log_x`, `log_y`, `xlabel`, `ylabel`, `latex`), `plot.update` changes some settings of an existing plot, `plot.get` returns them, `plot.panel` and `plot.edit` open the dialog. Hostile or huge input is refused with an error: the expression length, the number of samples (20,000), the points of the curve and the size of a data file are limited.
