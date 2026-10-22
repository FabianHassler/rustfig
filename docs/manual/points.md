# Points

These modes edit the points of lines, polygons and splines (a box, arc-box or picture has four corner points).

## Move point

Key `Shift+M`.

- Left, “move point”: press on a point, drag, release (or click, click). The line follows.
- Middle, “horiz/vert move”: the point moves only horizontally or vertically from where it was.
- Right: cancels.

The ends of an open line can be constrained with [Angle Geom](angle-geom.md). Moving a corner of a box stretches the lines attached to its edges ([Smart links](smart-links.md)). The Select mode's handles do the same.

## Add point

No key (xfig key Ctrl+A). Inserts a point into a line.

- Left, “break/add here”: press on a segment, drag the new point to where it should be and release.
- Middle, “add at end”: press near an end of an open line or spline; a new point is added beyond that end.
- Right: cancels.

## Delete point

Key `Shift+D`.

- Left, “delete point”: click a point of a line or spline.
- Middle, “delete in region”: mark a region; every point of lines and splines inside is deleted.

If the region holds no point, the message panel says so.
