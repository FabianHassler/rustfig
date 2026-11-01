# The window

RustFig keeps xfig's arrangement of the main window.

## Top strip

- **Menus**: File, Edit, View, Snap, Object (RustFig's own, for commands xfig only has as modes or keys) and Help. See [Menus](menus.md).
- **File name**: the current file; it is marked when there are unsaved changes. Click it to rename the file.
- **Message panel**: what to do next, errors and information. Click it to see the history of messages (also Help > Message history..., `app.messages`).
  - Each message has a level: information, warning (for example the notes after opening a file) or error (red). The window's **Info**, **Warning** and **Error** check boxes choose which levels are listed, **Copy** puts the listed messages on the clipboard one per line (warnings and errors start with their level), **Clear** empties the history and **Dismiss** (or Return) closes the window, as in xfig. Scripts use `app.messages` and `app.copy_messages` with `levels`, for example `["warning","error"]`.
- **Mouse indicator**: three boxes, one per mouse button, say what each does in the current mode and step. Hold Shift to see the Shift meanings. Hovering shows the same as text.
- **Units box**: shows the units (inches or centimetres, a scale); click it for the unit panel (Edit > Set units).

## Mode panel (left)

One button per mode: drawing modes first, then editing modes. Hover for the name and key. The same modes are available as keys, see [Keys](keys.md). The mode is highlighted while it is active. On a small screen the panel scrolls vertically (mouse wheel over it); the buttons keep their size.

At the top of the panel, which stays in place while the buttons scroll, is the **Find mode** field. Type part of a mode's name (upper or lower case; the name and xfig's long text are searched, so `arr` finds Add/remove arrows) and a short list shows the matches with each mode's key at the right, to learn it. Up and Down move in the list, Return or a click picks the mode exactly as its button does, Escape clears the field. Ctrl+J (Cmd+J on macOS; View › Find mode) puts the caret in the field from anywhere.

Below it, the **Favourites** strip holds up to eight small mode buttons of your own choice; it is hidden while empty. Right-click a mode button and choose Add to favourites (or Remove from favourites); a click on a favourite selects the mode like the panel button, and the same button shows as active. The favourites are kept in the settings file (`[rustfig] mode_favourites`). Scripts use `mode.favourite` and `mode.favourites`.

## Start screen

When RustFig starts, a card with its name, version and credits shows over the canvas for three seconds, as xfig shows its splash graphic; Help › About RustFig shows the same card, with the licence and the copyrights, until you click or press a key. The first key or click fades it out and still does its job. Start with `-nosplash`, or turn off the `splash` setting in Global settings, to skip it. Scripts can show it again with the `app.splash` command (Show Start Screen).

## Canvas

The drawing area with the page (page borders can be shown or hidden, View menu) and optional grid. Rulers at the top and left show the position; the scale follows the zoom. Scroll or use the rulers' arrows to pan; Z and Shift+Z zoom (see [Zoom and view](view.md)).

## Indicator panel (bottom)

Buttons for the attributes that new objects get and that the current mode needs: line width, line style, colours, fill, arrows, font and so on. Which buttons show depends on the mode, as in xfig. See [The indicator panel](attributes.md).

## Depth manager (right, optional)

Lists the depths in use with check boxes to show or hide them (View > Show depth manager).

Choosing a mode puts xfig's sentence for it, with its key, in the message panel (for example “Rectangular BOX drawing   (b)”). After opening a file the message reads `Current figure "name" (N objects)`. RustFig's own Trim button is the last of the editing buttons, so every xfig button keeps its place in the grid.

## Dialogs

A dialog taller than the window (Global settings on a small screen, for example) gets a vertical scroll bar: scroll with the mouse wheel or drag the bar to reach the rows and buttons below.
