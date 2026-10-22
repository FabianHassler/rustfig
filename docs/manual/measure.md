# Measuring

The three measure modes (the last three buttons of the editing column) report in the message panel; they change nothing in the drawing, add no undo step and leave the selection as it is. Lengths and areas appear in the drawing's units (inches or centimetres, scaled by the Unit/Scale setting), with one decimal; inches show fractions of 1/64 unless the rulers count in tenths. Every number below also comes from the commands `object.measure_angle`, `object.measure_length`, `object.measure_area` and `object.measure_reset` (see [Automation](automation.md)).

## Measure angle

Key Ctrl+G (the physical Ctrl key). Measures an angle between three points you click, or at a corner of a line or at the centre of an arc.

- Left, “first point”: click the first point of the angle. A rubber band follows the pointer. The next click (“angle tip”) is the corner, then the third click (“final point”, left button) measures the angle at the corner between the two lines and reports “Angle defined by three points: 45.00 Degrees”, between -180 and 180 degrees (the sign says which way it turns).
- Middle, “select & save”: click a vertex of a line or polygon, or one of the three defining points of an arc. The angle is reported and also stored as the **Rotation Angle** (0 to 180 degrees) of the Rotate modes. While the three points are being placed the middle button's label is “save angle”: the third point clicked with it also stores the angle.
- Right, “select object”: the same without storing. While the points are being placed the right button's label is “cancel”.

At a polyline corner the report is “Angle at polyline corner: ...”; at an arc “Angle at arc center: ...” (the angle the arc sweeps, positive or negative by its direction). At the first or last point of an open line there is no corner: “Can't compute angle at endpoint”. A polygon has a corner at every point, including the first. Escape also cancels the three points. A click on the same point twice is ignored.

## Measure length

Key Ctrl+N. Measures the length of lines, polygons, boxes, arcs, circles and ellipses.

- Left, “select object”: click the object; the message panel says for example “Length of polygon is 3.0 in”. For a circle it is the circumference, for an ellipse an approximation of the perimeter, for an arc the length along the arc. A rounded box (arcbox) gives its perimeter less the circumference of a circle with the corner radius, as xfig does.
- Middle, “select & add”: the same, and the length is added to a running total: “Length of box is 2.0 in, accumulated 5.0 in”.
- Right, “reset to 0”: sets the running total to 0 (“length reset to 0”).

Objects the mode cannot measure (text, splines, compounds) show no markers and are not found by a click. Objects on a hidden depth are not found either.

## Measure area

Key Ctrl+M. Measures the area of polygons, boxes, arcs, circles and ellipses; lines are not closed, so they have no area.

- Left, “select object”: click the object: “Area of box is 2.0 square in”. A circle or ellipse gives pi times the two radii; an arc counts as a sector (“sector”) for a pie arc and as the segment between arc and chord (“open arc”) for an open arc; a rounded box (arcbox) loses the area of a circle with the corner radius, as xfig does. A polygon whose sides cross itself is not measured correctly (as in xfig).
- Middle, “select & add”: the same, and the area is added to a running total: “Area of polygon is 2.0 square in, accumulated 3.0 square in”.
- Right, “reset to 0”: sets the total to 0 (“area reset to 0”). With Shift held the label reads “reset to +-0” and the total is reset to 0 and signed from now on (“signed area reset to 0”): an area is then negative when the polygon's points turn counter-clockwise on the screen (an arc by its direction), so overlapping and opposite parts cancel in the total. The ordinary right button turns the sign off again. Metric areas are given with three decimals, “square cm”.

Shift+left locates an object under another one (“locate object”, see [Mouse](mouse.md)); the next plain click measures the located object. Shift+middle does nothing.
