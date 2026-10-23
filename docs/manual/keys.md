# Keys

<!-- os: mac -->
RustFig understands two sets of keys: xfig's own, and the usual ones of macOS. xfig's set can be switched off in Edit > Global settings ("xfig keyboard shortcuts"). The macOS shortcuts use Cmd (written Ctrl in the tables below); xfig's Meta is the Option key. Where a table says "the physical Ctrl key", it means the Control key, not Cmd.

The menus show both keys of an entry where it has both, for example `⌥ U / ⌘ Z`: Option and Cmd are written ⌥ and ⌘ (with Control ⌃ and Shift ⇧), and punctuation keys are spelled out (`⌘ Quote` is Cmd and the apostrophe key, `⌘ Minus`, `⌘ Equals`).
<!-- os: linux windows -->
RustFig understands two sets of keys: xfig's own, and the usual ones of your system. xfig's set can be switched off in Edit > Global settings ("xfig keyboard shortcuts"). xfig's Meta is the Alt key.

Where both sets want the same key (Ctrl+S, Ctrl+C and so on), xfig's meaning wins while xfig's keys are on. The menus show both keys of an entry where it has both, for example `Alt+U / Ctrl+Z`; modifiers are written Ctrl, Alt and Shift, and punctuation keys are spelled out (`Ctrl+Quote` is Ctrl and the apostrophe key, `Ctrl+Minus`, `Ctrl+Equals`).
<!-- os: end -->

## Mode keys

The keys that choose a mode are shown in the mode sections: for example `C` Circle, `B` Box, `L` Polyline, `M` Move, `D` Delete. `W` Trim, split and extend (RustFig's own). Shift selects the variant: `Shift+C` Circle by diameter, `Shift+B` Arc-box. The Ctrl keys xfig uses for modes are:

| Key | Mode |
|---|---|
| Ctrl+S (physical Ctrl) | Scale |
| Ctrl+C | Copy |
| Ctrl+A | Add point |
| Ctrl+E | Edit attributes |
| Ctrl+R | Rotate clockwise |
| Ctrl+P | Picture |
| Ctrl+G | Measure angle |
| Ctrl+N | Measure length |
| Ctrl+M | Measure area |

Mode keys do nothing while a menu is open or while you type text.

In Help > Keys and the menus xfig's key comes first; the system's key is added only where it differs (a mode letter such as `C` is xfig's, so it appears once). On a Mac the Meta key is shown as ⌥ and the physical Ctrl key as ⌃; the words Meta and Alt are not used there.

The complete list, searchable, is in Help > Keys...: every command with its key in your system's scheme and in xfig's scheme, side by side (it shows the keys of the system you are on). Both sets work.

## xfig's menu keys (Meta = Option or Alt)

| Key | Action |
|---|---|
| Meta+N, O, M | New, Open, Merge |
| Meta+S, A | Save, Save As |
| Meta+X | Export; Shift+Meta+X exports at once with the last settings |
| Meta+Q | Exit |
| Meta+U | Undo |
| Meta+T | Paste |
| Meta+I | Search/Replace |
| Meta+D | Delete all |
| Meta+R or Ctrl+L | Redraw |
| Meta+C | Portrait/Landscape |
| Meta+B | Show page borders |
| Meta+Y | Show info balloons |
| Meta+G | Global settings |
| Shift+U | Set units |
| Shift+Z, Z | Zoom in, zoom out |
| Ctrl+Z (the physical Ctrl key) | Zoom to fit |

Meta+F, E, V and H open the File, Edit, View and Help menus; while a menu is open, the underlined letter of an entry picks it (for example `i` for Zoom In in the View menu). In the Help menu `x` opens this manual and `k` the key list. In the View menu `c` opens the character map (also Ctrl+Shift+M, even while typing).

## Platform keys

| Key | Action |
|---|---|
| Ctrl+N, O, S, Shift+S, Q | New, Open, Save, Save As, Quit |
| Ctrl+E, Ctrl+Shift+E | Export |
| Ctrl+Z, Ctrl+Shift+Z | Undo, Redo |
| Ctrl+V | Paste objects (the cut buffer) |
| Ctrl+=, Ctrl+-, Ctrl+0, Ctrl+1 | Zoom in, out, to fit, actual size |
| Ctrl+' , Ctrl+B | Show grid, show page border |
| Ctrl+F | Search/Replace |

## Other keys

- Arrow keys pan the canvas; Home returns to the origin.
- Escape works one level per press: it closes an open panel; otherwise it (1) cancels what is in progress (a shape being drawn is dropped, a move, paste or zoom box stops; typed text is finished), then (2) clears what the mode has collected (Chop's axe list, the located object, the rotate or flip centre, the selection), then (3) leaves the mode, as when RustFig starts. Edit > Select none still selects nothing.
- Backspace (or Delete) while drawing a line, polygon or spline removes the last point, like Shift+left.
- In Text mode: Left, Right, Home, End, Backspace and Delete edit, Enter ends the line and starts the next one below it. Shift extends the selection; word jumps and select all, copy, cut and paste follow the platform:
<!-- os: mac -->
  Option+arrows (words), Cmd+arrows (start, end), Cmd+A, Cmd+C, Cmd+X, Cmd+V.
<!-- os: linux windows -->
  Ctrl+arrows (words), Ctrl+A, Ctrl+C, Ctrl+X, Ctrl+V.
<!-- os: end -->
- Enter confirms and Escape cancels in most panels.
