# The indicator panel

The bottom strip holds the attributes that new objects get and that the current mode needs. As in xfig, which buttons appear depends on the mode: a drawing mode shows what that shape has; editing modes such as Update show all of them. Every button works with the mouse like xfig's: the left button opens a menu of choices, the middle button steps to the previous value and the right button to the next (the three words "Menu", "Dec/Prev" and "Inc/Next" under the strip remind you).

Buttons that exist (labels as in xfig):

| Button | What it sets |
|---|---|
| Zoom | the zoom factor |
| Grid Mode | grid style and spacing ([Snap and grid](snap-grid.md)) |
| Point Posn | how points snap while drawing ([Snap and grid](snap-grid.md)) |
| Depth | the depth (stacking order): lower is in front |
| Rotn Angle | the angle of the rotate modes |
| Num Sides | sides of a regular polygon |
| Vert Align, Horiz Align | alignment of the align mode |
| Angle Geom | constrains line directions ([Angle Geom](angle-geom.md)) |
| Smart Links | attached lines follow ([Smart links](smart-links.md)) |
| PenColor, FillColor | line (or text) colour and fill colour; the menu has xfig's standard colours, your own colours, the colours used lately, and **More…** (see Colours below) |
| Fill Style | none, solid, tints and the pattern fills |
| Arc Type | open or pie-wedge arcs |
| Line Width | in 1/80 inch; thickness 0 draws no line |
| Line Style | solid and the dash styles ([Line styles](line-styles.md)) |
| Tangent/Norm Length | [Tangent and normal](tangent-normal.md) |
| Join Style, Cap Style | corners and line ends (miter, round, bevel; butt, round, projecting) |
| Arrow Mode, Arrow Type, Arrow Size | arrowheads |
| Box Curve | corner radius of rounded boxes |
| Text Just, Text/Ellipse Angle, Text Flags, Text Size, Text Step, Text Font | text settings |

**Arrow Size** opens xfig's panel: **Absolute Values** or **Multiple of Line Width**, each with Thickness, Width and Length (0.1 to 10000; absolute values are in 1/80 inch). Only the set you chose is used for new arrows; with the multiples (xfig's start state: 1, 4 and 8) an arrow grows with the line width. **Set** (or Return) applies, **Cancel** keeps the old values. "Use current attributes as defaults" keeps the choice for the next start.

Number pop-ups (Line Width, Text Size, Depth, Rotn Angle, Num Sides, copies, Box Curve, Text Step, Zoom and others) work as xfig's: type or drag a value, then **Set** or Return applies it and closes the pop-up; **Cancel** closes it without a change. The Zoom pop-up also has **Integer zoom**: zoom values above 1 become whole numbers when you set them there (rounded) or use Fit (truncated).

**Grid Mode** has nine choices: none, four square grids and four **isometric** grids (vertical lines and lines at plus and minus 30 degrees; points snap to the crossings, [Snap and grid](snap-grid.md)). **Vert Align** and **Horiz Align** have xfig's seven choices: none, start (left or top), centre, end (right or bottom), distribute centres, distribute edges and abut ([Transform](transform.md)).

The Update Control box (in Update mode) says which attributes Update applies. Edit > Global settings has "Use current attributes as defaults" to keep the panel's values as the start values for new drawings ([Preferences](preferences.md)).

## Colours

Click PenColor or FillColor for the colour menu: Default, the 32 standard colours, the colours of the drawing and your own colours. Under them are the **recent colours** (the last twelve you used, kept in your settings, not in the drawing) and **More…**.

- **More…** (also Edit > Colours…) opens the Colours window:
  - Pick in the hue/saturation square and the hue strip, or type Red, Green, Blue and Hue, Saturation, Value.
  - Type a hex value (`#rrggbb` or `#rgb`) or an X11 colour name such as `Dark Orange` (case and blanks do not matter; matching names are offered while you type) into **Hex or name**.
  - **Use as pen colour** and **Use as fill colour** make the colour current; a colour that is not one of the 32 standard colours becomes a user colour of the drawing.
  - **Colours in this drawing** lists the user colours (numbers 32 to 543), saved in the Fig file. Click one to edit it, then **Change colour N** (every object that uses it changes), **Delete colour N** (refused while an object uses it) or **Add as new colour**. **Remove unused colours** drops those no object uses.
  - **Pick from screen** takes the colour of any pixel on the screen, also in other programs' windows, and shows it in the window; apply it with **Use as pen colour** or **Use as fill colour** (only then does it become a user colour of the drawing). It is an addition: xfig has nothing like it.
<!-- os: mac -->
    On macOS the system's colour sampler (a magnifier) opens: click a pixel to take it, Escape cancels. It needs no permission.
<!-- os: windows -->
    On Windows the colour under the pointer is shown live next to the button; click to take it (the click also reaches the window you click on), Escape cancels.
<!-- os: linux -->
    On Linux the desktop's colour picker opens through xdg-desktop-portal (Wayland and X11, GNOME and KDE provide it); where no portal runs, the message line says so.
<!-- os: end -->
    In the web app the button is greyed out: the browser's EyeDropper needs JavaScript.
  - **Keep for all drawings** stores the colour under a name in your settings, so it is offered in every drawing.

Scripts and MCP use the `color.*` commands (`color.parse`, `color.new`, `color.edit`, `color.delete`, `color.recent`, …).

## Styles

**View › Manage Styles…** (Ctrl+Y with xfig's keys on) keeps named sets of attributes in families, as xfig's style manager does, so a look you use again and again is one click away.

  - The window shows two lists side by side: **Family** (a name field, **Add**, **Delete**) and **Style** (a name field, **Add**, **Delete**, **Set**). Pick a family to see its styles.
  - A style holds the indicator's drawing attributes: depth, line width, style, join, cap, dash length and dot gap, pen and fill colour, fill style, box radius, arrows, the text font, size, angle, justification and flags, and the arc type.
  - Set the indicator panel as you want it, pick a family, type a name and press **Add** under Style (a style of the same name is replaced).
  - **Set** sets the indicator, so what you draw next looks like the style; objects already drawn are not changed (as in xfig). **Delete** removes the selected family or style. **Close** (or Escape) closes the window.
  - Limits: 32 families, 64 styles per family (xfig: 16 and 16), names of 1 to 64 characters.
  - Styles are kept in your settings file, for all drawings. They are never written to the .fig file, which stays as xfig reads it.

Scripts and MCP use `style.family_add`, `style.family_delete`, `style.family_rename`, `style.save`, `style.apply`, `style.rename`, `style.delete` and `style.list`, each with a `family` parameter.
