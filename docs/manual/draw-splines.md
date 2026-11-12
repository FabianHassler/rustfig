# Splines

A spline is a smooth curve through or near a series of control points. Fig files have three kinds, each open or closed; all six modes work the same way and differ in the curve they make.

- **Approximated splines** pass near the points (the points pull the curve). Modes **Approximated spline** and **Closed approximated spline**.
- **Interpolated splines** pass through every point. Modes **Interpolated spline** and **Closed interpolated spline**.
- **X-splines** let you choose per point how sharp or smooth the curve is (see below). Modes **X-spline** and **Closed X-spline**. This is the most flexible kind.

Buttons in all six modes: left adds points, middle draws freehand or finishes, right cancels. A closed spline joins its last point to the first. A closed spline needs three points (the middle button is dead until the last one is missing); an open spline needs two, otherwise the message panel says "Not enough points for spline" and you keep drawing. Shift+left removes the last point but never the first; Shift+middle finishes at the last point placed without adding the clicked one. [Angle Geom](angle-geom.md) can constrain the points.

## Approximated spline

Key `S`.

- Left, “first point”, then each next point.
- Middle, “freehand” to start freehand drawing; middle again, “final point”, finishes.
- Right: cancels.

## Closed approximated spline

Key `Shift+S`. As Approximated spline, but the curve closes on itself.

- Left, “first point”.
- Middle, “freehand”.

## Interpolated spline

Key `I`. The curve passes through each point you click.

- Left, “first point”.
- Middle, “freehand”.

## Closed interpolated spline

Key `Shift+I`.

- Left, “first point”.
- Middle, “freehand”.

## X-spline

No key and no button: type its name in **Go to mode** at the top of the mode panel (Tools > Go to mode..., Ctrl+J). The curve's shape at each control point is a number between -1 and 1: 0 makes a sharp corner, 1 a smooth curve that approximates the point, -1 a smooth curve through it. New points are smooth approximations; change them with Ctrl+Shift and a click on a point in an editing mode such as Move point (see *Shape factor on the canvas* below), or set the number in [Edit attributes](attribute-modes.md) (right button on a point of the spline).

- Left, “first point”.
- Middle, “freehand”.

An approximated or interpolated spline you have already drawn becomes an X-spline the same way: change one of its points with Ctrl+Shift and a click.

## Closed X-spline

No key and no button: type its name in Go to mode. As X-spline but closed; a closed approximated or interpolated spline becomes one with Ctrl+Shift and a click on a point.

- Left, “first point”.
- Middle, “freehand”.

## Shape factor on the canvas

This turns an approximated or interpolated spline into an X-spline, and changes the points of an X-spline. In the editing modes (Move point, Edit and the others, not the drawing modes), hold Ctrl and Shift and click a spline point: the left button makes the point more approximating (shape +0.04 up to 1), the right button more interpolating (-0.04 down to -0.5, xfig's `S_SPLINE_INTERP`), the middle button cycles through interpolating (-0.5), angular (0) and approximating (1). The spline becomes an X-spline. The end points of an open spline are refused ("Cannot edit end-points"); a click away from spline points says "Only spline points can be edited". The indicator shows "More approx / Cycle shapes / More interp" while both keys are held. Like xfig, the changed spline goes to the end of the list (it draws on top of its depth). Undo restores the old shape.

Ctrl is the Control key on every system, on macOS too.

## Changing a spline later

Use [Points](points.md) to move, add and delete control points, and [Convert line and spline](attribute-modes.md) to turn a polyline into a spline and back.
