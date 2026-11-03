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
| PenColor, FillColor | line (or text) color and fill color; the menu has xfig's standard colors, your own colors, the colors used lately, and **More…** (see Colors below) |
| Fill Style | none, solid, tints and the pattern fills |
| Arc Type | open or pie-wedge arcs |
| Line Width | in 1/80 inch; thickness 0 draws no line |
| Line Style | solid and the dash styles ([Line styles](line-styles.md)) |
| Tangent/Norm Length | [Tangent and normal](tangent-normal.md) |
| Join Style, Cap Style | corners and line ends (miter, round, bevel; butt, round, projecting) |
| Arrow Mode, Arrow Type, Arrow Size | arrowheads |
| Box Curve | corner radius of rounded boxes |
| Text Just, Text/Ellipse Angle, Text Flags, Text Size, Text Step, Text Font | text settings |

**Text Flags** shows one flag at a time as xfig does (`hidden=off`, `TeX=on`, `rigid=off`); the right or middle button shows the next or previous one, a click opens the panel that sets all three. **Text Size** shows two decimals (`12.00`). The number pop-ups (Depth, Rotation Angle, …) are live (see below).

**Text Font** opens the font chooser (also in the edit panel's Font row). The switch **PostScript / LaTeX** picks xfig's two lists: the 35 PostScript fonts, or the six LaTeX fonts (Default, Roman, Bold, Italic, Sans Serif, Typewriter). Every name is drawn in its own font (with only the built-in fonts, every name is drawn in the fallback and tagged "(substitute)"); the current font is marked with a dot and highlighted. Type in **Search fonts** to narrow the list (every word must occur in the name). **Size** sets the Text Size, and the preview line shows sample text in the font under the pointer (or the current font) at that size. Click a name to choose it.

**Which fonts are used.** Text is drawn with the fonts installed on your system: for each of xfig's 35 PostScript fonts RustFig looks for Times, Helvetica, Courier, Symbol, Zapf Dingbats, Palatino, Bookman, Century Schoolbook and so on (or their URW and TeX Gyre clones), then for the nearest serif, sans-serif or monospaced face, and last for its built-in fonts. The TeX Gyre fonts (metric clones of the PostScript fonts) are used when a TeX installation or system package provides them. If you have none, **Help > Install TeX Gyre fonts...** downloads them from CTAN (about 21 MB), checks the download's SHA-256 checksum, and keeps only the OpenType files and their license in RustFig's user data folder; a failed or canceled download leaves nothing behind, and the fonts are used from then on. Setting `FIG2DEV_BUILTIN_FONTS=1` makes RustFig use only its built-in fonts (the same drawing on every machine).

The **Arrow Size** button shows the arrow as it will look (current type, thickness, width and length, at a fixed scale, so a larger arrow looks larger; very large ones are cut at the button's edge) and **Box Curve** shows one corner of a box with the current radius, the number beside it; both follow the pop-up's slider. **Arrow Size** opens xfig's panel: **Absolute Values** or **Multiple of Line Width**, each with Thickness, Width and Length (0.1 to 10000; absolute values are in 1/80 inch). Only the set you chose is used for new arrows; with the multiples (xfig's start state: 1, 4 and 8) an arrow grows with the line width. The panel is live like the number pop-ups below, with a slider and number field per row: Return or a click outside keeps, Escape restores. "Use current attributes as defaults" keeps the choice for the next start.

Number pop-ups (Line Width, Text Size, Text Angle, Depth, Rotn Angle, Num Sides, copies, Box Curve, Text Step, Tangent Length) are small and stay out of the way. Each has a number field (xfig's ranges; the mouse wheel steps it) and a row of preset values, and where it helps a slider: Line Width, Box Curve, Text Size, Text Step and Tangent Length have one over the useful range. Angles have no slider but chips for 0, 15, 30, 45, 60, 90, 120, 180 and 270 degrees; Depth, Num Sides and the copy counts have -10/-1/+1/+10 buttons and chips instead. The value applies at once, so the indicator and the next object you draw follow while you drag or type. **Return** or a click outside keeps the value; **Escape** puts back the value the button had when you opened it. Nothing is added to the undo list, since the indicator is not part of the drawing. The Zoom pop-up keeps xfig's **Set** and **Cancel**. The Zoom pop-up also has **Integer zoom**: zoom values above 1 become whole numbers when you set them there (rounded) or use Fit (truncated).

**Grid Mode** has nine choices: none, four square grids and four **isometric** grids (vertical lines and lines at plus and minus 30 degrees; points snap to the crossings, [Snap and grid](snap-grid.md)). **Vert Align** and **Horiz Align** have xfig's seven choices: none, start (left or top), center, end (right or bottom), distribute centers, distribute edges and abut ([Transform](transform.md)).

The Update Control box (in Update mode) says which attributes Update applies, and a line below the buttons, "Update will change: …", spells them out. Edit > Global settings has "Use current attributes as defaults" to keep the panel's values as the start values for new drawings ([Preferences](preferences.md)).

## Colors

Click PenColor or FillColor for the color menu: Default, the 32 standard colors, the colors of the drawing and your own colors. Under them are the **recent colors** (the last twelve you used, kept in your settings, not in the drawing) and **More…**.

- **More…** (also Edit > User colors…) opens the Colors window:
  - Pick in the hue/saturation square and the hue strip, or type Red, Green, Blue and Hue, Saturation, Value.
  - Type a hex value (`#rrggbb` or `#rgb`) or an X11 color name such as `Dark Orange` (case and blanks do not matter; matching names are offered while you type) into **Hex or name**.
  - **Use as pen color** and **Use as fill color** make the color current; a color that is not one of the 32 standard colors becomes a user color of the drawing.
  - **Colors in this drawing** lists the user colors (numbers 32 to 543), saved in the Fig file. Click one to edit it, then **Change color N** (every object that uses it changes), **Delete color N** (refused while an object uses it) or **Add as new color**. **Remove unused colors** drops those no object uses.
  - **Pick from screen** takes the color of any pixel on the screen, also in other programs' windows, and shows it in the window; apply it with **Use as pen color** or **Use as fill color** (only then does it become a user color of the drawing). It is an addition: xfig has nothing like it.
<!-- os: mac -->
    On macOS the system's color sampler (a magnifier) opens: click a pixel to take it, Escape cancels. It needs no permission.
<!-- os: windows -->
    On Windows the color under the pointer is shown live next to the button; click to take it (the click also reaches the window you click on), Escape cancels.
<!-- os: linux -->
    On Linux the desktop's color picker opens through xdg-desktop-portal (Wayland and X11, GNOME and KDE provide it); where no portal runs, the message line says so.
<!-- os: end -->
    In the web app the button is grayed out: the browser's EyeDropper needs JavaScript.
  - **Keep for all drawings** makes the color a favorite: it is stored in your settings (under the name you type, or its hex code), offered in every drawing's color menus and listed in Global settings > Drawing > Your colors. **Keep all for all drawings** does this for every color of the open drawing that is not a favorite yet.

Scripts and MCP use the `color.*` commands (`color.parse`, `color.new`, `color.edit`, `color.delete`, `color.recent`, …).

## Styles

**View › Manage Styles…** (Ctrl+Y with xfig's keys on) keeps named sets of attributes in families, as xfig's style manager does, so a look you use again and again is one click away.

  - The window shows two lists side by side: **Family** and **Style**, each with what is saved; under each list, after a gap, a field for a new name and the buttons (**Add**, **Delete**; for styles also **Set**). Pick a family to see its styles.
  - A style holds the indicator's drawing attributes: depth, line width, style, join, cap, dash length and dot gap, pen and fill color, fill style, box radius, arrows, the text font, size, angle, justification and flags, and the arc type.
  - The name fields are for new entries only. Type a family name and press **Add** (or Return): the family appears in the list, selected, and the field empties. To add a style, set the indicator panel as you want it, pick a family, type a name and press **Add** under Style (a style of the same name is replaced). Clicking a name in a list selects it; it does not copy it into the field.
  - **Set** sets the indicator, so what you draw next looks like the style; objects already drawn are not changed (as in xfig). **Delete** removes the selected family or style. **Close** (or Escape) closes the window.
  - Limits: 32 families, 64 styles per family (xfig: 16 and 16), names of 1 to 64 characters.
  - Styles are kept in your settings file, for all drawings. They are never written to the .fig file, which stays as xfig reads it.

Scripts and MCP use `style.family_add`, `style.family_delete`, `style.family_rename`, `style.save`, `style.apply`, `style.rename`, `style.delete` and `style.list`, each with a `family` parameter.

## Hover preview

In Find mode, pointing at a choice in a pictorial panel (Line Style, Join, Cap, Arrow Type, Fill Style) or moving a number pop-up shows what **Update** would do to the matched objects, on the canvas, at once. Moving the pointer away, or closing the pop-up, puts the objects back; nothing is added to the undo list and nothing in the drawing changes until you press Update. With no matched objects (and in the other modes) there is nothing to preview.
