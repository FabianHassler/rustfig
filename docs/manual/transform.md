# Scale, align, flip, rotate

All of these act on the object you click; a compound is one object. With [Smart links](smart-links.md) on, attached line ends follow.

## Scale

No key (xfig key Ctrl+S). Resizes an object in proportion.

- Left, “scale box”: press on the object, drag, release. The fixed point is the bounding-box corner opposite the one nearest your click, and the object grows with the pointer's distance from it.
- Middle, “scale about center”: the same around the centre of the object.
- Right: cancels.

## Align

Key `A`. Aligns the parts of a compound with each other, or one object with another. The indicator's Vert Align and Horiz Align choose how the edges or centres line up.

- Left, “align compound”: click a compound; its members are aligned with each other.
- Right, “align to first”: click the object that stays, then click each object to align to it.

## Flip up/down

Key `F`. Mirrors an object about a horizontal line through the clicked point.

- Left, “flip”: flips the object.
- Middle, “copy & flip”: leaves the original and adds the flipped copy.

## Flip left/right

Key `Shift+F`. As above but mirrors about a vertical line.

- Left, “flip”.
- Middle, “copy & flip”.

## Rotate counter-clockwise

Key `Shift+R`. Rotates the clicked object by the indicator's Rotn Angle (set in the indicator panel).

- Left, “rotate object”.
- Middle, “copy & rotate”: rotates a copy.
- Shift reverses the direction.

## Rotate clockwise

No key (xfig key Ctrl+R). As above, the other way.

- Left, “rotate object”.
- Middle, “copy & rotate”.

The Object menu has the same operations for the selection (Transform and Arrange): rotate, flip, align, distribute, and the depth order (bring to front, send to back).
