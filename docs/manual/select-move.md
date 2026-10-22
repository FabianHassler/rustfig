# Move, copy, delete

xfig has no selection: you choose a mode first (Move, Copy, Delete, …) and then click the object it acts on. RustFig works the same way. To act on an object hidden under another one, Shift+left locates it first ([Mouse](mouse.md)). The cut buffer is filled by Copy mode's right button (“copy to cut buf”) or Delete mode's right button (“del to cut buf”), and emptied with Edit > Paste objects.

## Move

Key `M`. Moves one object, or a compound as a whole.

- Left, “move object”: press on an object, drag, release; or click it and click again at the new place.
- Middle, “horiz/vert move”: the same, but the move is restricted to horizontal or vertical, whichever is greater.
- Right, “move region”: mark a region with two clicks or a drag; every object completely inside moves, placed with the next click.

With [Smart links](smart-links.md) on, lines attached to a moved shape follow. Grid and object snaps apply.

## Copy

No key (xfig key Ctrl+C). As Move, but leaves the original.

- Left, “copy object”: press, drag, release (or click, click).
- Middle, “horiz/vert copy”.
- Right, “copy to cut buf”: puts the object in the cut buffer and on the system clipboard (as a figure other RustFig windows can paste, and as a picture for other programs); Edit > Paste objects attaches it to the pointer later ([Edit menu](menus.md)).
- While the copy is being placed the buttons read “place object” (left, one copy) and “array placement” (middle): with Num X Copies and Num Y Copies set (indicator fields `num_x_copies` and `num_y_copies`, 0 to 99), the middle button places a grid of copies spaced by the distance you moved, along x and y. With only one of them set to 1 you get a single extra copy that way; with both at 1, one along x and one along y; with larger numbers a columns-by-rows grid without the original's cell (command `edit.copy_array`).

## Delete

Key `D`.

- Left, “delete object”: click an object.
- Middle, “delete region”: mark a region; everything completely inside is deleted.
- Right, “del to cut buf”: deletes the object and keeps it on the clipboard (a cut).

Undo brings objects back.
