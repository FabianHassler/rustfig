# Keys

<!-- os: mac -->
RustFig has two sets of keys: the usual ones of macOS (with Cmd) and xfig's own (Meta is the Option key, Ctrl the Control key). The mode keys always work, and so do xfig's keys for commands that have no macOS key. xfig's other keys (Option+S for Save and so on) work when **xfig keyboard shortcuts in menus** is on in Edit > Global settings (off by default).

The menus show one key per entry: the macOS key (`⌘ Z` for Undo), or xfig's where the command has none, or xfig's for every entry when xfig's keys are on (Undo then shows `⌥ U`, and `⌘ Z` still undoes). Option and Cmd are written ⌥ and ⌘ (with Control ⌃ and Shift ⇧), with a thin space before the key, so `⌘ '` (Cmd and the apostrophe key), `⌘ -` and `⌘ =` stay readable. Option with a letter never picks that letter's mode.
<!-- os: linux windows -->
RustFig has two sets of keys: the usual ones of your system and xfig's own (Meta is the Alt key). The mode keys always work, and so do xfig's keys for commands that have no key of the system's and that take no key of the system's. xfig's other keys work when **xfig keyboard shortcuts in menus** is on in Edit > Global settings (off by default); where both sets want the same key (Ctrl+S, Ctrl+C and so on), xfig's meaning wins while they are on.

The menus show one key per entry: the system's (`Ctrl+Z` for Undo), or xfig's where the command has none, or xfig's for every entry when xfig's keys are on (for example `Alt+U / Ctrl+Z`). Modifiers are written Ctrl, Alt and Shift, and the key as its symbol (`Ctrl+'` is Ctrl and the apostrophe key, `Ctrl+-`, `Ctrl+=`). Alt with a letter never picks that letter's mode.
<!-- os: end -->

## Mode keys

Every mode with a key; the letters are xfig's and always work. Mode keys do nothing while a menu is open or while you type text. Modes without a key: Library (its button in the mode panel), Find properties (Tools > Find Properties...) and Edit bounding box (View > Bounding box). An ellipse is drawn with the circle keys and the middle button, and an object is opened or closed with `V` and the right button. The modes without a button (X-spline, Closed X-spline, Open/close and the two ellipse modes) are also found by name in Go to mode (Ctrl+J).

| Mode | Key |
|---|---|
| Circle (radius) | C |
| Circle (diameter) | Shift+C |
| Ellipse (radii) | E |
| Ellipse (diameter) | Shift+E |
| Approximated spline | S |
| Closed approximated spline | Shift+S |
| Interpolated spline | I |
| Closed interpolated spline | Shift+I |
| Polygon | P |
| Polyline | L |
| Box | B |
| Arc-box (rounded) | Shift+B |
| Regular polygon | Shift+P |
| Arc | R |
| Text | T |
| Plot | Y |
| Glue into compound | G |
| Break compound | Shift+G |
| Open compound | O |
| Join/split lines and splines | J |
| Chop | X |
| Trim, split and extend | W |
| Align | A |
| Move | M |
| Move point | Shift+M |
| Delete point | Shift+D |
| Delete | D |
| Update attributes | U |
| Flip up/down | F |
| Flip left/right | Shift+F |
| Rotate counter-clockwise | Shift+R |
| Convert line ↔ spline | V |
| Add/remove arrows | Shift+A |
| Add tangent/normal to curve | N |

xfig puts nine more modes on the Control key:

<!-- os: mac -->
They always work (macOS's own keys use Cmd).

| Mode | Key |
|---|---|
| Scale | ⌃ S |
| Copy | ⌃ C |
| Add point | ⌃ A |
| Edit attributes | ⌃ E |
| Rotate clockwise | ⌃ R |
| Picture | ⌃ P |
| Measure angle | ⌃ G |
| Measure length | ⌃ N |
| Measure area | ⌃ M |
<!-- os: linux windows -->
Those whose Ctrl key the system does not use always work; the others work while xfig's keys are on (otherwise the key keeps the system's meaning).

| Mode | Key | Works |
|---|---|---|
| Scale | Ctrl+S | with xfig's keys on (else Save) |
| Copy | Ctrl+C | with xfig's keys on (else copy) |
| Add point | Ctrl+A | with xfig's keys on (else select all) |
| Edit attributes | Ctrl+E | with xfig's keys on (else Export) |
| Rotate clockwise | Ctrl+R | always |
| Picture | Ctrl+P | with xfig's keys on (else Print) |
| Measure angle | Ctrl+G | always |
| Measure length | Ctrl+N | with xfig's keys on (else New) |
| Measure area | Ctrl+M | always |
<!-- os: end -->

## Platform keys

<!-- os: mac -->
| Key | Action |
|---|---|
| ⌘ N, ⌘ O, ⌘ S, ⇧⌘ S, ⌘ Q | New, Open, Save, Save As, Quit |
| ⌘ E, ⇧⌘ E | Export |
| ⌘ Z, ⇧⌘ Z | Undo, Redo |
| ⌘ V | Paste objects (the cut buffer) |
| ⌘ =, ⌘ -, ⌘ 0, ⌘ 1 | Zoom in, out, to fit, actual size |
| ⌘ ', ⌘ B | Show grid, show page border |
| ⌘ F | Search/Replace |
| ⌘ P | Print |
| ⌘ K | Enter point |
| ⌘ J | Go to mode |
| ⇧⌘ L | Redraw |
| ⇧⌘ M | Character map |
| ⇧⌘ K | Command search |
| ⇧⌘ P | Plot curve |
<!-- os: linux windows -->
| Key | Action |
|---|---|
| Ctrl+N, O, S, Shift+S, Q | New, Open, Save, Save As, Quit |
| Ctrl+E, Ctrl+Shift+E | Export |
| Ctrl+Z, Ctrl+Shift+Z | Undo, Redo |
| Ctrl+V | Paste objects (the cut buffer) |
| Ctrl+=, Ctrl+-, Ctrl+0, Ctrl+1 | Zoom in, out, to fit, actual size |
| Ctrl+', Ctrl+B | Show grid, show page border |
| Ctrl+F | Search/Replace |
| Ctrl+P | Print |
| Ctrl+K | Enter point |
| Ctrl+J | Go to mode |
| Ctrl+Shift+L | Redraw |
| Ctrl+Shift+M | Character map |
| Ctrl+Shift+K | Command search |
| Ctrl+Shift+P | Plot curve |
<!-- os: end -->

## xfig's menu keys

<!-- os: mac -->
Meta is the Option key (⌥), xfig's Ctrl the Control key (⌃).
<!-- os: linux windows -->
Meta is the Alt key.
<!-- os: end -->

Marked *always*: the command has no key of the system's, so xfig's key works whether xfig's keys are on or not. The others work while xfig's keys are on (Edit > Global settings).

<!-- os: mac -->
| Key | Action | Works |
|---|---|---|
| ⌥ N, ⌥ O | New, Open | with xfig's keys |
| ⌥ M | Merge | always |
| ⌥ S, ⌥ A | Save, Save As | with xfig's keys |
| ⌥ X | Export | with xfig's keys |
| ⌥⇧ X | Export at once with the last settings | always |
| ⌥ Q | Exit | with xfig's keys |
| ⌥ U | Undo | with xfig's keys |
| ⌥ T | Paste | with xfig's keys |
| ⌥ I | Search/Replace | with xfig's keys |
| ⌥ K | Spell check | always |
| ⌥ D | Delete all | always |
| ⌥ Z | Digitize | always |
| ⌥ P | Print | with xfig's keys |
| ⌥⇧ P | Print at once | always |
| ⌥ R or ⌃ L | Redraw | with xfig's keys |
| ⌥ C | Portrait/Landscape | always |
| ⌥ B | Show page borders | with xfig's keys |
| ⌥ L | Show line lengths | always |
| ⌥ Y | Show info balloons | always |
| ⌥ G | Global settings | always |
| Shift+U | Set units | always |
| Shift+Z, Z | Zoom in, zoom out | with xfig's keys |
| ⌃ Z | Zoom to fit | with xfig's keys |
| ⌃ K | Enter point | with xfig's keys |
| ⌃ Y | Manage Styles | always |
<!-- os: linux windows -->
| Key | Action | Works |
|---|---|---|
| Alt+N, O | New, Open | with xfig's keys |
| Alt+M | Merge | always |
| Alt+S, A | Save, Save As | with xfig's keys |
| Alt+X | Export | with xfig's keys |
| Shift+Alt+X | Export at once with the last settings | always |
| Alt+Q | Exit | with xfig's keys |
| Alt+U | Undo | with xfig's keys |
| Alt+T | Paste | with xfig's keys |
| Alt+I | Search/Replace | with xfig's keys |
| Alt+K | Spell check | always |
| Alt+D | Delete all | always |
| Alt+Z | Digitize | always |
| Alt+P | Print | with xfig's keys |
| Shift+Alt+P | Print at once | always |
| Alt+R or Ctrl+L | Redraw | with xfig's keys |
| Alt+C | Portrait/Landscape | always |
| Alt+B | Show page borders | with xfig's keys |
| Alt+L | Show line lengths | always |
| Alt+Y | Show info balloons | always |
| Alt+G | Global settings | always |
| Shift+U | Set units | always |
| Shift+Z, Z | Zoom in, zoom out | with xfig's keys |
| Ctrl+Z | Zoom to fit | with xfig's keys (else Undo) |
| Ctrl+K | Enter point | always (the system's key too) |
| Ctrl+Y | Manage Styles | always |
<!-- os: end -->

Meta with F, E, V and H opens the File, Edit, View and Help menus (the Tools menu has no letter); while a menu is open, the underlined letter of an entry picks it (for example `i` for Zoom In in the View menu). In the Help menu `x` opens this manual and `k` the key list. In the View menu `c` opens the character map (its key in the table above works even while typing). 
<!-- os: mac -->
⌘ J puts the caret in the Go to mode field of the mode panel, in both key schemes (xfig has no key there).
<!-- os: linux windows -->
Ctrl+J puts the caret in the Go to mode field of the mode panel, in both key schemes (xfig has no key there).
<!-- os: end -->

## Other keys

- Arrow keys pan the canvas; Home returns to the origin.
- Escape works one level per press: it closes an open panel; otherwise it (1) cancels what is in progress (a shape being drawn is dropped, a move, paste or zoom box stops; typed text is finished), then (2) clears what the mode has collected (Chop's axe list, the located object, the rotate or flip center), then (3) leaves the mode, as when RustFig starts.
- Backspace (or Delete) while drawing a line, polygon or spline removes the last point, like Shift+left.
- In Text mode: Left, Right, Home, End, Backspace and Delete edit, Enter ends the line and starts the next one below it. Shift extends the selection; word jumps and select all, copy, cut and paste follow the platform:
<!-- os: mac -->
  Option+arrows (words), Cmd+arrows (start, end), Cmd+A, Cmd+C, Cmd+X, Cmd+V.
<!-- os: linux windows -->
  Ctrl+arrows (words), Ctrl+A, Ctrl+C, Ctrl+X, Ctrl+V.
<!-- os: end -->
- Enter confirms and Escape cancels in most panels.
