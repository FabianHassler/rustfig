# Line styles

Line styles are chosen with the indicator's **Line Style** button: solid, dashed, dotted, dash-dotted, dash-double-dotted and dash-triple-dotted.

## One number per line

A Fig file keeps only one style value per line, in 1/80 inch: the dash length for the dashed styles, or the gap between dots for Dotted. There are no separate dash, gap and dot lengths in the format (an extension for them was considered and declined, so every Fig reader draws the same line). The rest of the pattern follows from that number v, as fig2dev draws it (and RustFig's canvas and exports do):

| Style | Pattern (v is the style value) |
|---|---|
| Dashed | dash v, gap v |
| Dotted | dot, gap v |
| Dash-dotted | dash v, gap v/2, dot, gap v/2 |
| Dash-double-dotted | dash v, gap 0.45v, dot, gap 0.33v, dot, gap 0.45v |
| Dash-triple-dotted | dash v, gap 0.4v, dot, gap 0.3v, dot, gap 0.3v, dot, gap 0.4v |

A dot is a very short dash.

## The two defaults

The Line Style panel keeps two defaults for new lines, as xfig: the **Default dash length** (used by the dashed styles) and the **Default dot gap** (used by Dotted). These supply the style value of a new line. To change the value of an existing line, open its edit panel ([Edit attributes](attribute-modes.md)), which shows the style value with the explanation above.
