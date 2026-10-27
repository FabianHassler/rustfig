# Libraries

A library is a folder of figures (`.fig` files) you can place in your drawing: electronic parts, flags, computers, flowchart blocks, maps and more. RustFig comes with xfig's libraries (about 2,500 figures in 30 libraries) and reads your own from a folder of yours.

## Library mode

Library mode button (the wide button at the bottom of the drawing modes) or `Shift+L`. The library panel opens.

1. **Load a Library**: choose a library in the *Library* menu (`Electronic / Schematic / Classic` is the library `Classic` in `Schematic` in `Electronic`). The *Status* line says how many objects it holds.
2. Click an object: its icon or name is picked, its name shows next to *Selected object*, and its comments (the text xfig shows under *Object comments*) appear below with a preview.
3. **Select object** (or double-click the object) closes the panel and puts the object in hand: it follows the pointer, its top-left corner at the pointer.
4. Place it, as below.

**Search**: type in the *Search* field to find figures in all libraries at once, not only the loaded one. Every word you type must occur in the figure's name or in its comments (upper and lower case do not matter); the results list each figure with its library, and those found by their comments say so. Click a result: its library is loaded and the figure picked, then proceed as above (double-click, or *Select object*). *Clear* returns to the library's own list. A long result list is cut; type more of the name.

Special (LaTeX) texts in a figure, such as `$\lambda/2$` or `$|0\rangle$`, show in the icons and the preview as a quick Unicode approximation (λ/2, |0⟩; Greek letters, common symbols, super- and subscripts, fractions); no TeX is run. The figure and what you place keep the LaTeX source.

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

The bundled libraries are in the `libraries` folder next to the program (or in the folders named in the environment variable `RUSTFIG_LIBRARIES`). 

In the web version the page serves the bundled libraries as static files (`cargo xtask web` copies `libraries/` into `dist/` and writes the index `libraries.json` there). The panel fetches the index when it opens and each figure the first time it is drawn, so the first view of a big library fills in while you watch ("Loading icons..."); a figure is fetched again only after the page is reloaded. The browser has no folder for your own libraries, so the *Source* field, *Copy to library* and a user library are desktop-only. Search finds figures by name; searching the comment lines needs the figures to have been fetched, so it is left out in the browser. Selecting an object before its preview has appeared can say "still loading, try again": wait for the preview and select again.

## Feynman diagrams

The `Feynman` library holds parts for Feynman diagrams, drawn for RustFig: fermion and anti-fermion lines (arrow in the middle), scalar (dashed), ghost (dotted), photon (wavy), W/Z boson (heavy wavy), gluon (curly) and graviton lines, each horizontal and one inch long (turn them with the Rotate modes); half-circle arcs of the same kinds for loops; a vertex dot, hatched and grey blobs; momentum arrows; and four complete examples (`example_ee_to_mumu`, `example_compton`, `example_self_energy`, `example_gluon_exchange`). To build a diagram, switch on grid snapping (1/4 inch), place a line part, then place the next part so that its end meets the first one; every line starts and ends on the grid, so parts join exactly. Put a vertex dot on each meeting point. Rotate or flip parts with the editing modes, and scale a line part to make it longer (the waves stretch with it). Label the lines with the Text mode in LaTeX special text (`$e^-$`, `$\gamma$`, `$\mu^+$`); the labels show as LaTeX in a PDF or TikZ export. Open an example as a starting point and change its labels.

## Quantum circuits

The `QuantumCircuits` library holds parts for quantum-circuit diagrams in the look of Qcircuit and quantikz, drawn for RustFig: a qubit wire and a classical wire (double line), single-qubit gate boxes (`H`, `X`, `Y`, `Z`, `S`, `T`, `R_x(theta)` and an empty box), a measurement meter, control dot, open control and target, complete `cnot`, `cz` and `swap` parts on two wires, multi-qubit gate boxes over two and three wires, initial-state and ket labels, a barrier and a classical control line, plus four examples (`example_bell`, `example_ghz`, `example_teleportation`, `example_qft`). Qubit wires are one inch long and end on the 1/4-inch grid; the wires of a circuit are 1/2 inch apart, so with grid snapping on the parts line up and join. Labels are LaTeX special text (`$H$`, `$|0\rangle$`): the canvas shows them typeset by TeX (see *LaTeX text* in Text and pictures), and a PDF or TikZ export as LaTeX; change them with the Text mode. Search the libraries for `quantum circuit` to list every part.

## Optics and lab

The `OpticsLab` library adds optical-table parts to xfig's own `Optics` library, drawn for RustFig the way physics papers draw setups: a laser, laser beams in xfig's red (solid, with an arrow, dashed, an expanded band and a diverging cone), convex, concave and cylindrical lenses, flat mirrors (plain and with a hatched back, perpendicular to the beam and at 45 degrees), a concave mirror, a dichroic mirror, beam splitter cube and plate, a polarizing beam splitter, half- and quarter-wave plates, polarizers, an iris, a coiled fibre and a fibre coupler, a photodiode, a detector, a CCD camera, acousto- and electro-optic modulators, an optical isolator, a prism, a grating, a cavity of two curved mirrors, a vacuum chamber, a cryostat, a viewport, a flange pair and a sample holder, plus three examples (`example_michelson`, `example_mach_zehnder`, `example_fibre_path`). Every part is one compound whose optical axis runs through its centre at a height on the 1/4-inch grid, and parts in the beam have their beam ports one inch apart on the grid: with grid snapping on, place parts at the same height and they line up; put the red beam parts between them. Rotate the 45-degree parts with the editing modes (they turn a beam from the left upwards), and flip the fibre coupler for the output end. Wave plates carry their labels as LaTeX special text (`$\lambda/2$`). Search the libraries for `optics lab` to list every part.

## Physics circuits

The `PhysicsCircuits` library holds lumped circuit elements the way physics papers draw them, filling the gaps of xfig's `Electronic` library: resistors (rectangle and zigzag), a capacitor, an inductor, Josephson junctions (cross in a box and plain cross), a SQUID loop, DC and AC voltage and current sources, a battery, a transmission-line segment, variable versions with an arrow (resistor, capacitor, inductor, Josephson junction), grounds (earth with three bars, chassis, protective earth, analog and digital ground), a transformer and a mutual inductance, a port, a terminal and a wire junction dot, plus three examples (`example_rlc`, `example_transmon`, `example_resonator`). Every part comes in one orientation (two-terminal parts horizontal, grounds pointing down); turn it with the Rotate modes. All two-terminal parts have their two pins exactly one inch apart on the 1/4-inch grid: with grid snapping on, place parts so that pins meet and join them with short wires drawn with the Polyline mode; put a junction dot where three wires meet. A ground has one pin, on top; a transformer has four pins on the grid. The examples carry LaTeX special-text labels (`$C_c$`, `$E_J$`): change them with the Text mode. Search the libraries for `circuit` to list every part.

## Commands

For scripts, the control channel and MCP: `library.list`, `library.source` (what `library_dir` names: a folder, or a list file with its entries), `library.use` (`path`: make a folder or list file the source), `library.search` (`query`, `limit`, `comments`: figures of all libraries by name and comments), `library.objects` (`comments: true` adds each figure's comments), `library.place` (`library`, `name`, `x`, `y` or `orig`, `ops`), `library.hand` (put the object in hand as Select object does), `library.transform`, `library.save`, `library.panel`. See [Automation](automation.md).

## Credits and licences

The figures belong to their authors, named in the READMEs in each library folder and in `ASSETS.md`; they are distributed with xfig under the xfig licence (the ERD and GUI libraries carry their own copy of it). The `Charts` library (released under the GPL) and two example drawings with restrictive notices are not bundled.
