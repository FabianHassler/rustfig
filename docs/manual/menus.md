# Menus

Entries xfig has are named as in xfig; RustFig's own entries follow them. An entry greyed out is not available in the current situation (nothing selected, nothing to undo).

## File

New, Open, Merge, Save, Save As, Export, Import picture (RustFig), Exit, then the recent files (numbered; pick with the number key). See [Files](files.md) and [Export](export.md).

## Edit

Undo, Paste objects, Search/Replace, Delete all, Global settings, Colours, Set units, Document settings (RustFig), Enter point, then Redo and Copy as picture. xfig has no selection, so there is no Cut, Copy or Select all: the modes act on the object you click, and the cut buffer is filled by Copy mode or Delete mode (right button).

- Undo and Redo have many levels.
- Paste objects attaches the cut buffer to the pointer as an outline, its top-left corner at the pointer. Left places the objects there (“place object”), middle at the position they were cut or copied from (“place at orig posn”), right or Escape cancels (“cancel paste”). One placement is one undo step. Scripts and MCP give `dx` and `dy` to `edit.paste` to place the objects at once, moved by that offset.
- Search/Replace finds text objects by their text and can replace it.
- Copy as picture (RustFig's addition) puts a picture on the system clipboard, for pasting into other programs: after choosing it, click what to copy — left an object, middle a region (two corners or a loop), right the whole figure; Escape cancels. The mouse indicator shows the three buttons meanwhile.

## View

Redraw, Portrait/Landscape, Zoom in, Zoom out, Zoom to fit canvas, Unzoom, Pan to origin, then the switches: Show page borders, Show depth manager, Show info balloons, Show grid, All depths on. See [Zoom and view](view.md).

## Snap

Hold and Release, then the object snaps for the next clicked point: Endpoint, Midpoint, Nearest, Focus, Normal, Tangent, Intersection, Diameter. See [Snap and grid](snap-grid.md).

## Object (RustFig)

Commands that xfig has only as modes or keys: Group and Ungroup, Update, Convert, Open/Close, Arrows, Transform (rotate, flip), Arrange (align, distribute, depth order), Measure and Edit Attributes. They act on the selection.

## Help

- RustFig Manual: this manual (xfig's "Xfig Reference").
- RustFig Reference (commands): every command with its id and shortcut, filterable. The same ids work in the [control channel and MCP](automation.md).
- About RustFig: version and licence; the xfig copyright holders.
