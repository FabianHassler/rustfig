# Tangent and normal

## Add tangent/normal to curve

Key `n`. Draws a short line touching a curve (a tangent) or crossing it at a right angle (a normal) at the point you click. Useful for marking slopes and construction lines.

- Left, “add tangent”: click a point on the curve; a line along the tangent is added, centred on that point.
- Middle, “add normal”: the same with the line perpendicular to the curve.
- Right: nothing.

The curve can be a line, polygon, box, spline, arc, circle or ellipse (also a turned ellipse). The new line is an ordinary three-point polyline in the current line attributes, depth and arrows, so you can change or delete it like any line. It is one undo step.

The length is the indicator's **Tangent/Norm Length** (in inches or centimetres as the drawing's units say; the new line is this long in total, centred on the point; start value 1.0; a button between Line Style and Join Style in this mode, steps of 0.1).

When several objects lie under the click, the search order is: ellipses and circles, then lines, then splines, then arcs, the last drawn first. At a corner of a polygon or a line's vertex the tangent is not defined: the message panel says "singularity, can't draw tangent/normal". The message "added tangent" or "added normal" confirms success. On splines the tangent is that of the short curve piece found at HIGH_PRECISION, as in xfig, so at the lowest point of a curve it is horizontal. A click that is not on any curve does nothing.
