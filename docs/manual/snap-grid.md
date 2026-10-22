# Snap and grid

Points you click are moved to a nearby position, so lines meet and shapes line up.

## Point Posn and Grid Mode

- **Point Posn** (indicator) sets the step that coordinates are rounded to: any (no rounding), 1/16, 1/8, 1/4, 1/2 or 1 inch (1, 2, 5 mm, 1, 2 cm in metric drawings). xfig starts with 1/16 inch, its "magnet".
- **Grid Mode** (indicator) chooses the grid spacing and style, or none: xfig's nine choices, four square grids and four isometric ones (`indicator.set` with `grid` and `grid_iso`; resource `startgridmode` 0 to 8). On an isometric grid the columns are cos 30 degrees of the spacing apart and every other column is shifted by half a spacing; points snap to those crossings (the spacing is the Point Posn step). The grid is shown on the canvas and points snap to it (View > Show grid, Ctrl+').

Pressing the middle button in the move modes (horiz/vert) constrains the move after snapping.

## Object snaps (Snap menu)

The Snap menu arms a snap for the next point you click:

| Snap | Snaps to |
|---|---|
| Endpoint | the end point or vertex of an object |
| Midpoint | the middle of a segment |
| Nearest | the nearest point on an object |
| Focus | the focus points of ellipses |
| Normal | the foot of a perpendicular from the previous point |
| Tangent | a tangent point from the previous point |
| Intersection | where two objects cross |
| Diameter | the opposite end of a circle's diameter |

**Hold** keeps the chosen snap after each point until you pick **Release**. The Snap Mode box next to the menu says which snap is armed.
