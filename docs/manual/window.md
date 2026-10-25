# The window

RustFig keeps xfig's arrangement of the main window.

## Top strip

- **Menus**: File, Edit, View, Snap, Object (RustFig's own, for commands xfig only has as modes or keys) and Help. See [Menus](menus.md).
- **File name**: the current file; it is marked when there are unsaved changes. Click it to rename the file.
- **Message panel**: what to do next, errors and information. Click it to see the history of messages.
- **Mouse indicator**: three boxes, one per mouse button, say what each does in the current mode and step. Hold Shift to see the Shift meanings. Hovering shows the same as text.
- **Units box**: shows the units (inches or centimetres, a scale); click it for the unit panel (Edit > Set units).

## Mode panel (left)

One button per mode: drawing modes first, then editing modes. Hover for the name and key. The same modes are available as keys, see [Keys](keys.md). The mode is highlighted while it is active.

## Canvas

The drawing area with the page (page borders can be shown or hidden, View menu) and optional grid. Rulers at the top and left show the position; the scale follows the zoom. Scroll or use the rulers' arrows to pan; Z and Shift+Z zoom (see [Zoom and view](view.md)).

## Indicator panel (bottom)

Buttons for the attributes that new objects get and that the current mode needs: line width, line style, colours, fill, arrows, font and so on. Which buttons show depends on the mode, as in xfig. See [The indicator panel](attributes.md).

## Depth manager (right, optional)

Lists the depths in use with check boxes to show or hide them (View > Show depth manager).

Choosing a mode puts xfig's sentence for it, with its key, in the message panel (for example “Rectangular BOX drawing   (b)”). After opening a file the message reads `Current figure "name" (N objects)`. RustFig's own Trim button is the last of the editing buttons, so every xfig button keeps its place in the grid.
