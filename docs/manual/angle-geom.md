# Angle Geom

Angle Geom (the "Angle Geom" button in the indicator panel) limits the directions of the line segments you draw, so that lines come out horizontal, vertical or at LaTeX's allowed slopes without careful aiming. xfig calls this point positioning for lines.

The button is shown in the modes where it applies: Polyline, Polygon, the spline modes, Move point and Add point. The six choices:

| Choice | Directions allowed |
|---|---|
| Unconstrained | any (the default) |
| Latex line | the slopes LaTeX's picture environment can draw for lines |
| Latex arrow | the slopes LaTeX allows for arrows (vectors) |
| Mount-hattan | horizontal, vertical and 45 degree diagonals |
| Manhattan | horizontal or vertical only |
| Mountain | 45 degree diagonals only |

How it works: the point you click, and the rubber band you see while moving, are placed in the nearest allowed direction from the previous point. When you move or add the end point of an open line, the constraint is taken from its neighbouring point. Points in the middle of a line, and points of closed shapes, are not constrained when moved.

xfig has no Shift key for 45 degree lines (Shift+left removes the last point while drawing), so there is none here: use Mount-hattan.

Angle Geom works together with the grid and Point Posn ([Snap and grid](snap-grid.md)).
