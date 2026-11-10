# Attribute modes

These modes change objects that already exist.

## Update attributes

Key `U`. Gives an object the current attributes of the [indicator panel](attributes.md) (colors, width, style, fill, arrows and so on). Which attributes are applied is set by the Update Control box shown in this mode: all, none or toggled flags, and individual ones. Under the buttons the line "Update will change: pen color, line width, …" lists the attributes whose flags are on, so you can read what a click will do without checking every button (script: `indicator.update_summary`).

- Left, “update object”: click an object.
- Middle, “update settings”: the reverse: the clicked object's attributes become the indicator's current ones.
- Right, “update region”: update every object completely inside a region.


## Edit attributes

No key (xfig key Ctrl+E). Opens the edit panel for an object with all of its attributes, text, points and comments. The panel is live: every change you make shows on the canvas at once, and Done (Ctrl+Return) keeps the whole edit as one undo step however many changes you made. Cancel (Escape) undoes everything since the panel opened. xfig's Apply button is left out: changes are applied as you make them.

Number fields of the panel (Width, Dash length or Dot gap, Corner radius, Depth, Text Size and Angle, an ellipse's Angle, the arrow Thick, Width and Length, a picture's Scale) are buttons showing the value. Click one for a small pop-up with the number field, preset values and, where a range helps, a slider (angles have preset chips for 0 to 270 degrees; Depth has -10/-1/+1/+10 buttons). The object follows as you drag or type; Return or a click outside keeps the value, Escape puts back the value the pop-up opened with. Point coordinates stay plain fields.

Pointing at a choice in a pop-up grid (Line style, Cap, Join, Fill style, Pen and Fill color, Forward and Backward arrow type) shows it on the object at once without changing the drawing; moving away puts the object back, a click keeps the choice. The arrow shows even while its **on** box is off.

- Line style, cap style, join style and the arrow types are pictorial menus: the button shows the picture of the current value, its pop-up lists every value as a picture (the same pictures as the attribute panel). Every row of the panel has the same height, labels sit on the left and controls in one column, and the point table has headings (#, X, Y, Shape) over its columns.
- Fill style (indicator and edit panel): the edit panel's "Fill style" row is a button showing the current fill; it opens the same choice panel as the indicator (NONE, shades and tints of the fill color, patterns) and a label beside it says "No fill", "Filled 50 %" or "Pattern 3" as xfig's panel does. For the fill colors black, white and default the menu has no tints, as in xfig (a tint set earlier becomes the full color).
- Points of a polyline, polygon or spline are a table: edit the X and Y of a row, Insert adds a point before that row, Delete removes it (the object keeps its minimum number of points), Add point appends one. Open Paste points, type or paste a list (one point per pair of numbers: `x y`, `x, y` or `(x, y)`, in the units shown in the panel) and choose Replace points or Append points. X-splines have a Shape column with each point's shape factor (-0.5 interpolated, 0 angular, 1 approximated; xfig's panel scale runs from -1 to 1).
- A compound has its own panel, see [Compounds](compounds.md).

- Left, “edit object”: click an object.
- Middle, “edit Main comment”: edits the figure's own comment and its LaTeX preamble (RustFig's addition, see [LaTeX text](draw-text-pictures.md)). Shift+middle on an object shows its comments in the message panel.
- Right, “edit point”: on a point of a spline, edits that point's shape factor (end points of an open spline cannot be edited).


## Convert line ↔ spline

Key `V`. Turns a polyline into a spline through its points, or a spline into a polyline. Polygons and closed splines go with it. A spline that was turned into a line comes back as the same kind (approximated, interpolated or X-spline) when the next line is turned into a spline; the first conversion makes an interpolated spline. A box becomes a rounded box (arc-box) with the radius of the indicator's Box Curve, and a rounded box becomes a plain box.

- Left, “line/spline, box/arc-box”: click the object.
- Right, “open/closed”: click a point (vertex) of a line, polygon or spline: a closed object opens there, an open one closes. A polygon opens at the clicked point: the open line starts at that point and ends at the point before it (clicking the first point of a polygon opens it at the second, as xfig does). A click on an edge between points does nothing.

## Open/close

No key. Closes an open line or spline (a polyline becomes a polygon) or opens a closed one.

- Left, “open/close”: click the object.

## Add/remove arrows

Key `Shift+A`. Arrowheads are shaped by the indicator's Arrow Type, Arrow Size and Arrow Mode.

- Left, “add arrow”: click on the first or last point of a line, arc or open spline (xfig's point search; a click anywhere else does nothing); an arrowhead is added at that end.
- Middle, “delete arrow”: click on the end point to remove the arrowhead there.
- Right, “reverse arrows”: swaps the arrowheads of the object's two ends.
