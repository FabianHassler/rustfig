# Join and split

## Join/split lines and splines

Key `J`. Joins two lines (or two splines) into one, or splits one in two.

- Left, “Join Lines/Splines”: click the end point of the first line; the indicator then reads “Choose next Line” (“Choose next Spline”); click the end of the second. The two become one, the ends meeting at the joined points. The lines may be joined in any of the four ways their ends can meet. Clicking both ends of one object closes it into a polygon or closed spline.
- Middle, “Split Line/Spline”: click a point or segment of a line or spline; it is cut there into two pieces. A polygon, box or closed spline is opened at the cut instead.
- Right: cancels a join in progress.

Arrowheads at the join or cut are removed. The first end picked is marked with a magenta marker. Only lines (or only splines) can be picked in the first step; to cut a line at an arbitrary point without a second object use this mode or [Chop](chop.md).
