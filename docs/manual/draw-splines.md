# Splines

A spline is a smooth curve through or near a series of control points. Fig files have three kinds, each open or closed; all six modes work the same way and differ in the curve they make.

- **Approximated splines** pass near the points (the points pull the curve). Modes **Approximated spline** and **Closed approximated spline**.
- **Interpolated splines** pass through every point. Modes **Interpolated spline** and **Closed interpolated spline**.
- **X-splines** let you choose per point how sharp or smooth the curve is (see below). Modes **X-spline** and **Closed X-spline**. This is the most flexible kind.

Buttons in all six modes: left adds points, middle draws freehand or finishes, right cancels. A closed spline joins its last point to the first. Shift+left removes the last point; Shift+middle finishes at the last point placed without adding the clicked one. [Angle Geom](angle-geom.md) can constrain the points.

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

No key. The curve's shape at each control point is a number between -1 and 1: 0 makes a sharp corner, 1 a smooth curve that approximates the point, -1 a smooth curve through it. New points are smooth approximations; change them later with [Edit attributes](attribute-modes.md) (right button on a point of the spline).

- Left, “first point”.
- Middle, “freehand”.

## Closed X-spline

No key. As X-spline but closed.

- Left, “first point”.
- Middle, “freehand”.

## Shape factor on the canvas

In the editing modes (not the drawing modes), hold Ctrl and Shift and click a spline point: the left button makes the point more approximating (shape +0.04 up to 1), the right button more interpolating (-0.04 down to -1), the middle button cycles through interpolating (-1), angular (0) and approximating (1). The spline becomes an X-spline. The end points of an open spline are refused ("Cannot edit end-points"); a click away from spline points says "Only spline points can be edited". The indicator shows "More approx / Cycle shapes / More interp" while both keys are held. Undo restores the old shape.

<!-- os: mac -->
On macOS use Cmd+Shift+click: Ctrl+click is the right button there.
<!-- os: linux windows -->
On Linux and Windows the keys are Ctrl+Shift.
<!-- os: end -->

## Changing a spline later

Use [Points](points.md) to move, add and delete control points, and [Convert line and spline](attribute-modes.md) to turn a polyline into a spline and back.
