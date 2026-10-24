# RustFig manual

RustFig is a Rust reimplementation of xfig, the classic vector drawing program for figures in papers and documents. It looks and behaves like xfig where that still makes sense, and adds modern conveniences (multi-level undo, a modern Select tool, automation) where xfig's way no longer fits. This manual is part of the program: it works offline and you can search it with the box above.

## Start here

- [Getting started](getting-started.md): draw your first figure in a few minutes.
- [The window](window.md): menus, mode panel, canvas, rulers, indicator panel, message panel.
- [The mouse](mouse.md): xfig uses three mouse buttons; here is how to get them on any computer, and what Shift does.
- [Keys](keys.md) and [Menus](menus.md): every shortcut and menu entry.

## Drawing

- [Shapes](draw-shapes.md): circles, ellipses, boxes, rounded boxes, regular polygons, arcs, polylines and polygons.
- [Splines](draw-splines.md): approximated, interpolated and X-splines, open and closed.
- [Text and pictures](draw-text-pictures.md).

## Editing

- [Move, copy, delete](select-move.md), [Scale, align, flip, rotate](transform.md).
- [Points](points.md): move, add and delete the points of a line.
- [Compounds](compounds.md), [Join and split](join-split.md), [Chop](chop.md), [Trim, split and extend](trim.md), [Tangent and normal](tangent-normal.md), [Measuring](measure.md).
- [Attribute modes](attribute-modes.md): update, edit, convert, open/close, arrows.
- [Libraries](libraries.md), [Plotting a curve](plot.md).

## Attributes and helpers

- [The indicator panel](attributes.md), [Line styles](line-styles.md), [Angle Geom](angle-geom.md), [Smart links](smart-links.md).
- [Snap and grid](snap-grid.md), [Zoom and view](view.md).

## Files and output

- [Files](files.md): open, save, merge, autosave and recovery.
- [Export](export.md): PDF, SVG, EPS, PNG and many more formats.
- [Print](print.md): page setup, preview, printer and batch.
- [Preferences](preferences.md) and [Automation](automation.md) (control channel, MCP, command line).
- [Finding help](help-tools.md): command search, F1 context help, the mouse cheat sheet, what's new.

## Where xfig's own manual went

xfig's Help menu opens an HTML reference, man pages and a How-To guide. In RustFig the Help menu opens this manual instead (Help > RustFig Manual). The command list (Help > RustFig Reference) shows every command with its id and shortcut. The command-line parts of xfig's man pages are in [Automation](automation.md).

Parts of this manual are adapted from xfig's HTML manual (xfig licence, see LICENSE-xfig).
