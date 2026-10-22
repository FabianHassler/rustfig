# Chop

Chop cuts objects where other objects cross them, like a knife along a ruler. xfig has the basic mode; RustFig adds trimming, splitting and extending as commands. The indicator panel shows Arc Type: a chopped arc becomes open or pie-wedge pieces.

## Chop

Key `x`. xfig's chop: you choose **axe** objects (the knives) and a **log** object (what gets cut). The log is cut at every point where it crosses an axe, and the pieces become separate objects with the log's attributes.

- Left, “Select axe object”: click an object to make it an axe; click it again to take it out. Any number of axes can be chosen. Axes are marked in orange (filled markers and a dashed box). Once there is an axe, the object under the pointer that the middle button would cut is marked in green.
- Middle, “Select log object”: click the object to cut; it is chopped by every axe at once.
- Right, “Clear axe list”: forgets all axes.

Lines, polygons, boxes, rounded boxes, arcs, circles, ellipses, splines and compounds can be logs (a compound chops every member). Lines, polygons, boxes, arcs, circles, ellipses, splines and compounds can be axes. A closed shape needs two crossings to be cut. Circles chop into pie wedges, ellipses into pieces drawn as X-splines, since the Fig format has no elliptical arcs; arrowheads stay only at the original ends. One chop is one undo step.

Trimming, splitting a line at a point and extending a line have a mode of their own: [Trim, split and extend](trim.md).
