# Points

These modes edit the points of lines, polygons and splines (a box, arc-box or picture has four corner points).

## Move point

Key `Shift+M`.

- Left, “move point”: press on a point, drag, release (or click, click). The line follows.
- Middle, “horiz/vert move”: the point moves only horizontally or vertically from where it was.
- Right: cancels.

The ends of an open line can be constrained with [Angle Geom](angle-geom.md). Moving a corner of a box stretches the lines attached to its edges ([Smart links](smart-links.md)).

## Add point

No key (xfig key Ctrl+A). Inserts a point into a line.

- Left, “break/add here”: click on a segment; the new point follows the pointer, with the line drawn through it, until you click again (“place new point”) where it should be. Pressing, dragging and releasing does the same in one move. Right cancels.
- Middle, “add at end” (RustFig's addition): click near an end of an open line or spline; the new end point follows the pointer, and a left click (“place new point”) places it beyond that end. A drag works too. xfig adds at an end with the left button on an end point; in RustFig that is the middle button's job.
- Right: cancels.

## Delete point

Key `Shift+D`.

- Left, “delete point”: click a point of a line or spline.
- Middle, “delete in region”: mark a region; every point of lines and splines inside is deleted.

If the region holds no point, the message panel says so.
