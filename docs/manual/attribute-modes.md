# Attribute modes

These modes change objects that already exist.

## Update attributes

Key `U`. Gives an object the current attributes of the [indicator panel](attributes.md) (colours, width, style, fill, arrows and so on). Which attributes are applied is set by the Update Control box shown in this mode: all, none or toggled flags, and individual ones.

- Left, “update object”: click an object.
- Middle, “update settings”: the reverse: the clicked object's attributes become the indicator's current ones.
- Right, “update region (drag: lasso)”: update every object completely inside a region.

Object > Update from Current Attributes (Ctrl+U) updates the selection.

## Edit attributes

No key (xfig key Ctrl+E). Opens the edit panel for an object with all of its attributes, text, points and comments. Done or Apply keeps the changes, Cancel (Escape) restores the object.

- Left, “edit object”: click an object.
- Middle, “edit Main comment”: edits the figure's own comment. Shift+middle on an object shows its comments in the message panel.
- Right, “edit point”: on a point of a spline, edits that point's shape factor (end points of an open spline cannot be edited).

Double-clicking an object in Select mode opens the same panel.

## Convert line ↔ spline

Key `V`. Turns a polyline into a spline through its points, or a spline into a polyline. Polygons and closed splines go with it.

- Left, “spline<->line”: click the object.
- Right, “open<->closed”: opens a closed object or closes an open one.

## Open/close

No key. Closes an open line or spline (a polyline becomes a polygon) or opens a closed one.

- Left, “open/close”: click the object.

## Add/remove arrows

Key `Shift+A`. Arrowheads are shaped by the indicator's Arrow Type, Arrow Size and Arrow Mode.

- Left, “add arrow”: click near the end of a line, arc or open spline; an arrowhead is added at that end.
- Middle, “delete arrow”: removes the arrowhead at that end.
- Right, “reverse arrows”: swaps the arrowheads of the object's two ends.
