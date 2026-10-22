# Chop

Chop cuts objects where other objects cross them, like a knife along a ruler. xfig has the basic mode; RustFig adds trimming, splitting and extending as commands.

## Chop

Key `x`. xfig's chop: you choose **axe** objects (the knives) and a **log** object (what gets cut). The log is cut at every point where it crosses an axe, and the pieces become separate objects with the log's attributes.

- Left, “Select axe object”: click an object to make it an axe; click it again to take it out. Any number of axes can be chosen. Axes are marked in orange (filled markers and a dashed box). Once there is an axe, the object under the pointer that the middle button would cut is marked in green.
- Middle, “Select log object”: click the object to cut; it is chopped by every axe at once.
- Right, “Clear axe list”: forgets all axes.

Lines, polygons, boxes, rounded boxes, arcs, circles, ellipses, splines and compounds can be logs (a compound chops every member). Lines, polygons, boxes, arcs, circles, ellipses, splines and compounds can be axes. A closed shape needs two crossings to be cut. Circles chop into pie wedges, ellipses into pieces drawn as X-splines, since the Fig format has no elliptical arcs; arrowheads stay only at the original ends. One chop is one undo step.

## Trim, split and extend

Shift+click in Chop mode locates an object, as in every picking mode ([Mouse](mouse.md)), so Chop keeps xfig's three buttons only. RustFig's additions are commands:

- `object.trim` removes the piece you click, the part of the object between its crossings with the axe objects (or with every other object when no axe is chosen). The rest stays as separate pieces.
- `object.split_at` cuts a line, polygon, arc or spline in two at a point; a closed figure is opened there.
- `object.extend` lengthens the nearest end of a line to the nearest boundary (the axe objects, or every other object).

They can be scripted (`object.trim`, `object.split_at`, `object.extend`; `object.cut_all` cuts all selected objects against each other), so they can be scripted ([Automation](automation.md)).
