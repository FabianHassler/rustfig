# Chop

Chop cuts objects where other objects cross them, like a knife along a ruler. xfig has the basic mode; RustFig adds trimming, splitting and extending on the Shift key.

## Chop

Key `x`. xfig's chop: you choose **axe** objects (the knives) and a **log** object (what gets cut). The log is cut at every point where it crosses an axe, and the pieces become separate objects with the log's attributes.

- Left, “Select axe object”: click an object to make it an axe (a guide box marks axes); click it again to take it out. Any number of axes can be chosen.
- Middle, “Select log object”: click the object to cut; it is chopped by every axe at once.
- Right, “Clear axe list”: forgets all axes.

Lines, polygons, boxes, rounded boxes, arcs, circles, ellipses, splines and compounds can be logs (a compound chops every member). Lines, polygons, boxes, arcs, circles, ellipses, splines and compounds can be axes. A closed shape needs two crossings to be cut. Circles chop into pie wedges, ellipses into pieces drawn as X-splines, since the Fig format has no elliptical arcs; arrowheads stay only at the original ends. One chop is one undo step.

## Shift variants

With Shift held, the same buttons do these RustFig additions (the indicator shows them while Shift is down):

- Shift+left, “trim piece”: removes the piece you click, the part of the object between its crossings with the axe objects (or with every other object when no axe is chosen). The rest stays as separate pieces.
- Shift+middle, “split here”: cuts the clicked line, polygon, arc or spline in two at the point you click. No axe is needed; a closed figure is opened there.
- Shift+right, “extend line”: lengthens the nearest end of the clicked line to the nearest boundary (the axe objects, or every other object).

Each is also a command (`object.trim`, `object.split_at`, `object.extend`; `object.cut_all` cuts all selected objects against each other), so they can be scripted ([Automation](automation.md)).
