# Compounds

A compound groups objects so that they act as one: move, copy, rotate and scale treat it as a single object. Compounds can contain compounds.

## Glue into compound

Key `G`. Collects objects into a compound.

- Left, “tag object”: click an object to tag it (it shows as selected); click it again to untag. Dragging on empty canvas tags everything inside the rectangle.
- Middle, “tag region”: click two corners or draw a loop; everything inside is tagged.
- Right: with two or more objects tagged ("compound tagged"), makes the compound. While a region is being marked, cancels.

Object > Group (Ctrl+G) does the same for the current selection.

## Break compound

Key `Shift+G`. Takes a compound apart.

- Left, “break compound”: click a compound; its parts become separate objects.
- Middle, “break and tag”: the same, and the parts stay tagged (selected) for a following Glue.
- Right, “break all levels”: breaks the compound and every compound inside it.

## Open compound

Key `O`. Edits the inside of a compound without breaking it. After opening, every mode works on the objects inside; the rest of the drawing is gray and cannot be touched.

- Left, “open compound”: click a compound.
- Middle, “open, keep visible”: the same, and the rest of the drawing stays visible in gray as a reference.

A "Close Compound" button appears while one is open; opening several in turn goes deeper. Saving asks whether to save only the visible part or everything. Undo steps through opening and closing too.

## Edit a compound

The edit panel of a compound (Edit mode, or Edit attributes, Ctrl+E) has, as in xfig:

- Top left corner and Bottom right corner of the compound's bounds. Changing a corner moves the compound; its size stays.
- Dimensions (width and height). Changing them rescales the compound about its top left corner; the bottom right corner follows.
- Depths: the smallest and largest depth inside, and the Collapse Depths button, which gives every object inside (also in compounds inside it) the smallest depth. Without that button the depths stay as they are.
- Number of objects, and one text field for each text directly inside the compound.

Changes show on the canvas as you make them; Done is one undo step, Cancel restores the compound. The corners are the compound's bounds as the export gives them, so they include the width of the lines and the text.
