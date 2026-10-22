# Keys

<!-- os: mac -->
RustFig understands two sets of keys: xfig's own, and the usual ones of macOS. xfig's set can be switched off in Edit > Global settings ("xfig keyboard shortcuts"). The macOS shortcuts use Cmd (written Ctrl in the tables below); xfig's Meta is the Option key. Where a table says "the physical Ctrl key", it means the Control key, not Cmd.

The menus show the key that applies.
<!-- os: linux windows -->
RustFig understands two sets of keys: xfig's own, and the usual ones of your system. xfig's set can be switched off in Edit > Global settings ("xfig keyboard shortcuts"). xfig's Meta is the Alt key.

Where both sets want the same key (Ctrl+S, Ctrl+C and so on), xfig's meaning wins while xfig's keys are on. The menus show the key that applies.
<!-- os: end -->

## Mode keys

The keys that choose a mode are shown in the mode sections: for example `C` Circle, `B` Box, `L` Polyline, `M` Move, `D` Delete. Shift selects the variant: `Shift+C` Circle by diameter, `Shift+B` Arc-box. The Ctrl keys xfig uses for modes are:

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

Meta+F, E, V and H open the File, Edit, View and Help menus; while a menu is open, the underlined letter of an entry picks it (for example `i` for Zoom In in the View menu). In the Help menu `x` opens this manual.

## Platform keys

| Key | Action |
|---|---|
| Ctrl+N, O, S, Shift+S, Q | New, Open, Save, Save As, Quit |
| Ctrl+E, Ctrl+Shift+E | Export |
| Ctrl+Z, Ctrl+Shift+Z | Undo, Redo |
| Ctrl+X, C, V, D | Cut, Copy, Paste, Duplicate |
| Delete or Backspace | Delete the selection |
| Ctrl+A, Escape | Select all, select none |
| Ctrl+G, Ctrl+Shift+G | Group, break compound |
| Ctrl+U | Update from current attributes |
| Ctrl+L, Ctrl+R | Rotate counter-clockwise, clockwise |
| Ctrl+Shift+H, Ctrl+Shift+V | Flip left/right, up/down |
| Ctrl+Shift+Up and Down, Ctrl+Up and Down | Bring to front, to back, forward, backward |
| Ctrl+=, Ctrl+-, Ctrl+0, Ctrl+1 | Zoom in, out, to fit, actual size |
| Ctrl+' , Ctrl+B | Show grid, show page border |
| Ctrl+F | Search/Replace |
| Ctrl+M | Measure |

## Other keys

- Arrow keys pan the canvas; Home returns to the origin.
- Escape closes an open panel, cancels the operation in progress, or (nothing else going on) selects nothing.
- In Text mode: Left, Right, Home, End, Backspace and Delete edit, Enter ends the line and starts the next one below it.
- Enter confirms and Escape cancels in most panels.
