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
- Middle, “Ellipse center”: press at the centre; the radii follow the pointer. Either button finishes it.

## Ellipse (diameter)

Key `Shift+E`.

- Left, “Circle diameter”: a circle by its diameter.
- Middle, “Ellipse corner”: press at one corner and drag to the opposite corner of the ellipse's box.

## Polygon

Key `P`. A closed shape with straight sides.

- Left, “first point”: click the first point, then each next point. Shift+left removes the last point.
- Middle, “freehand”: start a freehand path; move the pointer and the path follows. Middle again ends it ("final point"). When drawing with straight sides, middle at a point is the final point.
- Right: cancels while drawing.

The polygon closes itself when you finish. Angle Geom can constrain the points ([Angle Geom](angle-geom.md)).

## Polyline

Key `L`. An open line of straight segments (the same buttons as Polygon).

- Left, “first point”, then “next point” for each further point.
- Middle, “freehand” to start a freehand line; “final point” ends the line.
- Right, “single point”: before a line is started, enters a single dot; while drawing, cancels.

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
