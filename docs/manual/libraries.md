# Libraries

A library is a folder of figures (`.fig` files) you can place in your drawing: electronic parts, flags, computers, flowchart blocks, maps and more. RustFig comes with xfig's libraries (about 2,500 figures in 30 libraries) and reads your own from a folder of yours.

## Library mode

Library mode button (the wide button at the bottom of the drawing modes) or `Shift+L`. The library panel opens.

1. **Load a Library**: choose a library in the *Library* menu (`Electronic / Schematic / Classic` is the library `Classic` in `Schematic` in `Electronic`). The *Status* line says how many objects it holds.
2. Click an object: its icon or name is picked, its name shows next to *Selected object*, and its comments (the text xfig shows under *Object comments*) appear below with a preview.
3. **Select object** (or double-click the object) closes the panel and puts the object in hand: it follows the pointer, its top-left corner at the pointer.
4. Place it, as below.

**Search**: type in the *Search* field to find figures in all libraries at once, not only the loaded one. Every word you type must occur in the figure's name or in its comments (upper and lower case do not matter); the results list each figure with its library, and those found by their comments say so. Click a result: its library is loaded and the figure picked, then proceed as above (double-click, or *Select object*). *Clear* returns to the library's own list. A long result list is cut; type more of the name.

*View* switches between *Icon View* (thumbnails; *Icon size* 40 to 120) and *List View* (names). Both are remembered (preferences `icon_view`, `library_icon_size`). *Cancel* closes the panel.

## Placing

While an object is in hand, the mouse indicator shows:

- Left, “place object”: places a copy at the pointer and stays in library mode, so you can place more. The copy is one compound, selected; each placement is one undo step.
- Middle, “new object”: opens the library panel again to choose another object.
- Right, “cancel library”: leaves library mode without placing.
- Shift+left, “place and edit”: places the object and opens its edit panel.
- Shift+middle, “change draw mode”: shows a box the size of the object instead of the object itself (faster on slow screens), and back.
- Shift+right, “place at orig posn”: places the object where it is in the library file.

Keys while the object is in hand (xfig's): `r` turns it clockwise, `l` counter-clockwise, `h` flips it left to right, `v` up to down, `s` shrinks it by a tenth, `Shift+S` enlarges it by a tenth (all about the object's centre). `Escape` cancels.

Colours the library figure defines are added to the drawing's colours. Pictures a library figure uses stay in the library folder; the figure keeps pointing at them.

## Your own libraries

Your library folder is the preference `library_dir` (default `~/xfiglib`; [Preferences](preferences.md), `-library_dir` on the command line). Every folder in it that holds `.fig` files is a library; folders inside a folder are libraries of their own. Copy figures there with any file manager, or use *Copy to library* in the panel: give the figure a name (and a folder inside your library folder if you like); the selected objects, or the whole drawing if nothing is selected, are saved as a `.fig` file there. A comment typed in the figure's comments dialog becomes the *Object comments*.

### A list of library folders

Like xfig, `library_dir` may name a plain text file instead of a folder. Each line holds a library folder and, after a space, the name to show for it (`/home/me/electronics  My circuits`); without a name the folder's own name is used. Lines starting with `#` and blank lines are ignored, `~` is your home folder, and a relative folder is taken from the folder the list file is in. Every folder named becomes one library, its sub-folders libraries of their own (`My circuits / Analog`), in the order of the file. *Copy to library* saves into the first folder of the list. The *Source* field at the top of the panel takes a folder or a list file; *Use* checks it, remembers it as `library_dir` and reads the libraries again.

### Stop

A big library is read while the panel is open: the library folders are scanned in the background and the icons are drawn a few at a time, so the window stays usable. *Stop* (shown while this goes on) ends it: the scan keeps what it found so far and icons not yet drawn stay blank. *Resume* draws them.

The bundled libraries are in the `libraries` folder next to the program (or in the folders named in the environment variable `RUSTFIG_LIBRARIES`). The web version has no file system and lists none.

## Commands

For scripts, the control channel and MCP: `library.list`, `library.source` (what `library_dir` names: a folder, or a list file with its entries), `library.use` (`path`: make a folder or list file the source), `library.search` (`query`, `limit`, `comments`: figures of all libraries by name and comments), `library.objects` (`comments: true` adds each figure's comments), `library.place` (`library`, `name`, `x`, `y` or `orig`, `ops`), `library.hand` (put the object in hand as Select object does), `library.transform`, `library.save`, `library.panel`. See [Automation](automation.md).

## Credits and licences

The figures belong to their authors, named in the READMEs in each library folder and in `ASSETS.md`; they are distributed with xfig under the xfig licence (the ERD and GUI libraries carry their own copy of it). The `Charts` library (released under the GPL) and two example drawings with restrictive notices are not bundled.
