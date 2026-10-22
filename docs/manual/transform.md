# Scale, align, flip, rotate

All of these act on the object you click; a compound is one object. With [Smart links](smart-links.md) on, attached line ends follow.

## Scale

No key (xfig key Ctrl+S). Resizes an object in proportion.

- Left, “scale box”: press on the object, drag, release. The click picks the nearest of the eight handles of the bounding box. At a corner the fixed point is the opposite corner and the object grows in proportion with the pointer's distance from it. At the middle of an edge only the size across that edge changes (the width for the left and right edges, the height for the top and bottom edges), about the opposite edge (xfig's one-dimension box scale).
- Middle, “scale about center”: the same around the centre of the object.
- Right: cancels.

## Align

Key `A`. Aligns the parts of a compound with each other, or one object with another. The indicator's Vert Align and Horiz Align choose how the edges or centres line up.

The seven choices (each of the two indicators): **none**; **start**, **centre**, **end** (left, centre, right or top, centre, bottom: the edges or centres line up with those of the whole group); **distribute centres** (the centres of the objects are evenly spaced, the outer two stay); **distribute edges** (equal gaps between the objects, the outer two stay); **abut** (the objects are put edge to edge, starting from the first). Horizontal and vertical are independent. Command `arrange.align` (`h`, `v`: none, left, center, right, distribute_centres, distribute_edges, abut).

- Left, “align compound”: click a compound; its members are aligned with each other.
- Middle, “align canvas”: every object in the figure is aligned to the page (the paper size, turned for landscape) by the indicator's Vert Align and Horiz Align (command `arrange.align_canvas`). Distribute and abut do not apply to the canvas.
- Right, “align to first”: click the object that stays, then click each object to align to it.

## Flip up/down

Key `F`. Mirrors an object about a horizontal line through the clicked point.

- Left, “flip”: flips the object.
- Middle, “copy & flip”: leaves the original and adds the flipped copy.
- Right, “set anchor”: marks a point; flips then mirror about the anchor instead of the clicked point. Right again, “unset anchor”, removes it. Choosing a mode also removes it.

## Flip left/right

Key `Shift+F`. As above but mirrors about a vertical line.

- Left, “flip”.
- Middle, “copy & flip”.
- Right, “set anchor” / “unset anchor”, as above.

## Rotate counter-clockwise

Key `Shift+R`. Rotates the clicked object by the indicator's Rotn Angle (set in the indicator panel).

- Left, “rotate object”.
- Middle, “copy & rotate”: rotates a copy. The indicator's Num Copies (`num_copies`, 1 to 99) makes that many copies, the first turned by the angle, the second by twice the angle and so on, all about the same centre. The Num Copies button appears in the rotate modes (xfig's `I_ROTATE`): click it for a number field, right button increases, middle button decreases.
- Right, “set center”: marks the point to rotate about; right again, “unset center”, removes it. Without a centre the object turns about its own middle.
- Shift+left locates an object ([Mouse](mouse.md)); Shift+right sets the centre like the right button, as in xfig. To turn the other way use Rotate clockwise.

## Rotate clockwise

No key (xfig key Ctrl+R). As above, the other way.

- Left, “rotate object”.
- Middle, “copy & rotate” (Num Copies as above).
- Right, “set center” / “unset center”.


## Array copies

In Copy mode the indicator shows **Num X Copies** and **Num Y Copies** (0 to 99; right button increases, middle decreases). With the middle button (“copy array”) the copy is placed in that many columns and rows, spaced by the distance between the two clicked points.
