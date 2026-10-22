# Trim mode

## Trim, split and extend

RustFig's own mode (xfig has none), the button next to Chop in the editing column, key `w`, also in the Tools menu. It acts on the object you click, using every other object in the drawing as the cutting edges. The indicator panel shows Point Posn (the split point snaps to the grid as set there) and Arc Type (a cut arc becomes open or pie pieces).

- Left, “trim piece”: removes the piece you click, the part of the object between its crossings with the other objects. The rest stays as separate pieces. If the object has no piece between two crossings, nothing happens and the message line says so.
- Middle, “split at point”: cuts a line, polygon, arc or spline in two at the point you click; a closed figure is opened there.
- Right, “extend nearest end”: lengthens the end of the line nearest to the click to the nearest boundary, the first other object it meets.

Lines, polygons, boxes, arcs, circles, ellipses, splines and compounds can be clicked; they show markers while the mode is active. Shift+click locates an object, as in every picking mode ([Mouse](mouse.md)). Each action is one undo step.

The icon shows the idea: a solid line that ends at a cutting edge and goes on as a row of dots, the extension.

The same operations are commands (`tool.trim`, `object.trim`, `object.split_at`, `object.extend`; `object.cut_all` cuts all selected objects against each other), so they can be scripted ([Automation](automation.md)). To cut an object at every crossing with chosen axe objects use [Chop](chop.md).
