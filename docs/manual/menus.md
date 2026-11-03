# Menus

Entries xfig has are named as in xfig; RustFig's own entries follow them. An entry grayed out is not available in the current situation (nothing selected, nothing to undo).

## File

Sections Open (New, Open, Merge, Open and Merge with preview, Digitize), Save (Save, Save As, Saved versions), Output (Export, Print), then New window and Exit, and the recent files (numbered; pick with the number key). See [Files](files.md) and [Export](export.md).

## Edit

Sections Undo (Undo, Redo), Paste (Paste objects, Paste text, Copy as picture), Text (Search/Replace, Spell check, native app only), then Delete all and Settings (Global settings, User colors, Set units, Page setup). Find Properties, Enter point and the plots are in the Tools menu. xfig has no selection, so there is no Cut, Copy or Select all: the modes act on the object you click, and the cut buffer is filled by Copy mode or Delete mode (right button).

- Undo and Redo have many levels.
- Paste objects attaches the cut buffer to the pointer as an outline, its top-left corner at the pointer. Left places the objects there (“place object”), middle at the position they were cut or copied from (“place at orig posn”), right or Escape cancels (“cancel paste”). One placement is one undo step. If the system clipboard holds a figure copied in another RustFig window, that is pasted instead of the cut buffer; if it holds an image (PNG, for example from a browser or a screenshot), it is pasted as a picture object, saved as `clipboard-<number>.png` next to the figure (in the temporary folder while the figure has no file name) and referenced by that path like any picture in a .fig file. The web version has only the cut buffer. Scripts and MCP give `dx` and `dy` to `edit.paste` to place the objects at once, moved by that offset.
- Search/Replace finds text objects by their text and can replace it.
- Copy as picture (RustFig's addition) puts a picture on the system clipboard, for pasting into other programs: after choosing it, click what to copy — left an object, middle a region (two corners or a loop), right the whole figure; Escape cancels. The mouse indicator shows the three buttons meanwhile.

## View

Sections Drawing (Manage Styles, Redraw, Portrait/Landscape), Zoom (Zoom in, Zoom out, Zoom to fit canvas, Unzoom, Pan to origin), Character map, and Show (the switches: page borders, grid, LaTeX text, depth manager, all depths on, info balloons, line lengths, vertex numbers, crosshair, autorefresh). See [Zoom and view](view.md).

## Snap

Hold and Release, then the object snaps for the next clicked point: Endpoint, Midpoint, Nearest, Focus, Normal, Tangent, Intersection, Diameter. See [Snap and grid](snap-grid.md).

## Object (RustFig)

Commands that xfig has only as modes or keys: Group and Ungroup, Update, Convert, Open/Close, Arrows, Transform (rotate, flip), Arrange (align, distribute, depth order), Measure and Edit Attributes. They act on the selection.

## Tools

RustFig's own menu, right after View. Find: Find Properties (see [Text](draw-text-pictures.md)) and Go to mode (Ctrl+J; puts the caret in the Go to mode field of the mode panel). Add to the drawing: Import picture, Capture Screen (native app only) and Enter point. Plots: Plot curve and Edit plot (see [Plot](plot.md)).

## Help

- RustFig Manual: this manual (xfig's "Xfig Reference").
- RustFig Reference (commands): every command with its id and shortcut, filterable. The same ids work in the [control channel and MCP](automation.md).
- Install TeX Gyre fonts: downloads the TeX Gyre fonts from CTAN into RustFig's user data folder (native app; see [the font chooser](attributes.md)).
- About RustFig: version and license; the xfig copyright holders.
