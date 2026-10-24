# Finding help

Four helpers in the Help menu and on the keyboard.

## Command search

Help > Command search... (`help.command_search`, `Ctrl+Shift+K`; on macOS `⌘ ⇧ K`) opens a small window with a text field. Type part of a command's name, its id or its menu path ("arrange", "export", "edit.undo"): the matching commands are listed with their keys, best match first. Commands that are not available now (nothing selected, nothing to undo) are greyed. The arrow keys move through the list, Enter (or a click) runs the highlighted command, Escape closes the window. The window finds the commands of the menus and also RustFig's own dialogs, such as Keys... and Global settings...

`Ctrl+K` is not used for the search: in xfig it opens the keyboard panel (Enter point, `app.keyboard_entry`), and RustFig keeps that key.

## F1: help for what is under the pointer

Press `F1` while the pointer is over a mode button, over an attribute button in the indicator panel (each opens the page of its own topic, for example Line Style opens Line styles and Zoom opens Zoom and view), over the depth manager, the rulers or the units box (The window), or with a dialog open. The manual opens at the page that explains it. Elsewhere F1 opens the manual's first page.

## Mouse buttons

Help > Mouse buttons... (`app.mouse_sheet`) lists every mode with what its left, middle and right button do, taken from the mouse-function indicator itself (so it always agrees with it), and how to press the middle and right buttons without a three-button mouse: on macOS Option+click is the middle button and Ctrl+click (or a two-finger click) the right one; on Linux and Windows Alt+click or both buttons together is the middle button; on a touch screen a long press is the right button. See also [The mouse](mouse.md).

## What's new

Help > What's new... (`app.whats_new`) lists what changed in this version. The page opens by itself once after RustFig is updated (not on the very first start); RustFig remembers the version it last showed it for in the settings file.
