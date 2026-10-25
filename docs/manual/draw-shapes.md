# Shapes

Drawing modes make new objects with the attributes shown in the [indicator panel](attributes.md) (colours, line width, fill, depth and so on). Change those first, or change the object afterwards with [Update](attribute-modes.md). Coordinates snap to the grid and to object snaps ([Snap and grid](snap-grid.md)).

In every shape mode you can press, drag and release, or click once and click again to finish. The right button cancels a shape in progress. The mouse indicator reads, for each mode, the words quoted below.

## Circle (radius)

Key `C`. Draws a circle from its centre.

- Left, “Circle center”: press at the centre, drag out to the radius and release (while dragging the indicator says “set radius”).
- Middle, “Ellipse center”: the same, but draws an ellipse whose radii follow the pointer's horizontal and vertical distance.
- Right, “circle by 3 points”: click three points on the circle.

## Circle (diameter)

Key `Shift+C`. Draws a circle from one end of its diameter.

- Left, “Circle diameter”: press at one end, drag to the opposite end.
- Middle, “Ellipse corner”: draws an ellipse inside the box spanned by the two corners.
- Right, “circle by 3 points”: click three points on the circle.

## Ellipse (radii)

Key `E`. Same buttons as Circle (radius), but the ellipse is the main shape.

- Left, “Circle center”: a circle from its centre.
- Middle, “Ellipse center”: press at the centre; the radii follow the pointer. A left or middle click finishes it (“Ellipse corner”), as in xfig, or release the middle button after dragging.

## Ellipse (diameter)

Key `Shift+E`.

- Left, “Circle diameter”: a circle by its diameter.
- Middle, “Ellipse corner”: press at one corner and drag to the opposite corner of the ellipse's box.

## Polygon

Key `P`. A closed shape with straight sides.

- Left, “first point”: click the first point, then each next point. Shift+left removes the last point; Shift+middle finishes at the last point placed without adding the clicked one.
- Middle, “freehand”: start a freehand path; move the pointer and the path follows. Middle again ends it ("final point"). When drawing with straight sides, middle at a point is the final point.
- Right: cancels while drawing.

As in xfig, the middle button does nothing until only the last point is missing, so a polygon always has at least three points; Enter or a double click with fewer keeps you drawing. Shift+left never removes the first point (with one point it adds a point). The polygon closes itself when you finish. Angle Geom can constrain the points ([Angle Geom](angle-geom.md)).

## Polyline

Key `L`. An open line of straight segments (the same buttons as Polygon).

- Left, “first point”, then “next point” for each further point.
- Middle, “freehand” to start a freehand line; “final point” ends the line.
- Right, “single point”: before a line is started, enters a single dot; while drawing, cancels.
- Shift+left, “del point”, or Backspace: removes the last point. Escape drops the whole line. Shift+middle, “finish here”: ends the line at the last point placed, without adding the clicked point.
- Shift+left before the first point, “dimension line”: starts a dimension line (xfig's trigger, `d_line.c`). Click its second point (“final point”; Shift+left removes the first). The result is a compound of the line with arrows at both ends, a box in the middle with the length in the figure's units and scale, and a tick at each end; the compound's comment reads “Dimension line: 2.0 in”. The settings are the indicator's `dimline` group, edited in the “Dimension line” button's window (shown in Polyline mode; xfig's “Dimension Line Settings”: line thickness, style and colour; box thickness and colour; the two arrow types, length and width; ticks and their thickness; text font, size and colour; actual length or fixed text; decimals; OK stores, Cancel drops) — (thickness, arrows, ticks, box and text colours, font, decimals, fixed text). When the line is scaled, or an open dimension line is closed again after its main line was edited, the length, box and ticks are recomputed (`object.recompute_dimension`). Command `shape.dimension_line`.

The length in the dimension line follows xfig: in inches it is a whole number with a fraction (for example 2 1/4), otherwise a decimal. The Dimension line button's settings window shows a live example line ("Dimension line example") that follows the font, arrow types (pictures as in the arrow menu, or "none" per end) and the other settings. The example draws plain triangle arrowheads.

### Lengths while drawing

While a line, box, circle or ellipse is being drawn, or an object moved or copied, the message panel shows the length, dx, dy and angle of the segment from the fixed point, for example “2 points, Length = 1.0 in, dx = 1.0 in, dy = 0.0 in (0.0 deg)”. While a box or rounded box is drawn the message reads “Width = …, Height = …, Factor = …”, and a regular polygon “Length = …, dx = …, dy = … (angle) deg”. With Show Lengths on (command `view.toggle_lengths`, resource `showlengths`) the triangle of dx and dy is drawn at the segment and its two sides are labelled in red.

Other messages follow xfig as well. Moving or adding a point of a line or spline shows both neighbour lengths and offsets (“Len 1 = ..., Len 2 = ..., dx1 = ...”); moving the middle point of an arc adds its radius (“Rad = ...”); scaling a box or a compound shows “Width = ..., Length = ..., Factor = ...”. **Show crosshair** (View menu, Edit > Global settings, command `view.toggle_crosshair`, resource `crosshair`, option `-crosshair`) draws full-window red lines through the pointer while a shape is being drawn or an object moved, scaled or reshaped.

### Typed coordinates

Command `tool.point` (xfig's keyboard point entry) gives the next point as text instead of a click: `1.5, 2` (ruler units; fractions as `1-1/2`), `r1, r-0.5` or `r1 0.5` (relative to the current point: the last point of the line, or where the operation started), `a` for absolute, or polar `2<30` (length and angle in degrees, `2<0.5p` in multiples of pi, `2<1r` in radians; from the current point). The point is used exactly, without snapping.

In the app, **Edit > Enter point…** (Ctrl+K; physical Ctrl+K with xfig's keys) opens a small window with a text field: type a point and press Enter for the left button (the Left, Middle and Right buttons act as that mouse button), and the point goes to the current mode. A bad entry shows its error and stays in the field; the last ten entries are listed under History, click one to type it again.

## Box

Key `B`. A rectangle.

- Left, “corner point”: press at one corner, drag to the opposite corner.
- Middle, “center point”: press at the centre; the box grows around it. Shift+middle makes a square.

## Arc-box (rounded)

Key `Shift+B`. A rectangle with rounded corners; the radius is the indicator's Box Curve value. Buttons as for Box.

- Left, “corner point”.
- Middle, “center point” (Shift+middle: square).

## Regular polygon

Key `Shift+P`. A polygon with equal sides; the number of sides is the indicator's Num Sides.

- Left, “center point”: press at the centre, drag to a corner.
- Middle, “one side”: press at one end of a side and drag to the other end.

## Arc

Key `R`. A circular arc. The indicator's Arc Type chooses open or pie-wedge, and Arrow Mode adds arrowheads.

- Left, “first point”: click three points on the arc (first, middle, final).
- Middle, “center point”: first the centre, then the start point, a direction point and the end angle ("first point", "direction", "final angle" in the indicator).
- Right: cancels.
