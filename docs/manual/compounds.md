# Compounds

A compound groups objects so that they act as one: move, copy, rotate and scale treat it as a single object. Compounds can contain compounds.

## Glue into compound

Key `G`. Collects objects into a compound.

- Left, “tag object”: click an object to tag it (it shows as selected); click it again to untag. Dragging on empty canvas tags everything inside the rectangle.
- Middle, “tag region (drag: lasso)”: click two corners or draw a loop; everything inside is tagged.
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
