# Smart links

Smart links keep lines attached to shapes. When you move a shape, the ends of lines that touch it move with it, so a diagram's connectors stay connected. Nothing is stored in the file: a line end counts as attached when it lies within a few pixels (45 Fig units, about 0.04 inch) of the shape's outline at the moment you start the operation.

## Turning it on

The indicator's **Smart Links** button has three choices:

- **Off** (the default): nothing follows, as in plain editing.
- **Move**: an attached end point moves with the shape.
- **Slide**: the end point moves and the point before it slides so that the last segment keeps its horizontal or vertical direction. Good for right-angle connectors.

The button shows in Move, Copy, Move point, Scale, Flip, Rotate and Align modes.

## What attracts lines (targets)

Boxes, rounded boxes and pictures (the edges), compounds (anywhere inside their box), and, within 45 Fig units of the outline: circles, ellipses, arcs, polygons, closed splines and text boxes.

## What follows (followers)

Open polylines and open splines. A line attached to two shapes that move together follows at both ends. Only the end points are linked, not points in the middle.

## Operations that carry the links

Move, Copy (the attached lines are copied with the shape; the original line's end follows the copy as in xfig), rotate, flip, scale, align, the arrow-key nudge command, and Move point on a box corner (resizing the box stretches the attached lines with its edges; this last one is a RustFig extension). The Move, Copy, Move point and Scale previews show every follower.

Undo reverts the shape and its lines in one step.
