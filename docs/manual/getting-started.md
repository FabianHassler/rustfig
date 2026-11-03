# Getting started

## Your first figure

1. Pick a mode from the mode panel on the left, for example **Box** (key `B`). The message panel at the top says what to do next, and the mouse indicator shows what each mouse button does right now.
2. Press the left button on the canvas, drag, and release. The box is drawn with the current attributes shown in the indicator panel at the bottom (line width, colours, fill and so on).
3. Pick **Move** (key `M`), click the box and click again where it should go. Or use **Select** and drag it.
4. Change how things look: set an attribute in the indicator panel, then use **Update attributes** (key `U`) on an object, or double-click an object with Select to open its edit panel.
<!-- os: mac -->
5. Press Option+U or Cmd+Z to undo. RustFig keeps many undo levels.
<!-- os: linux windows -->
5. Press Alt+U or Ctrl+Z to undo. RustFig keeps many undo levels.
<!-- os: end -->
6. File > Save As writes a `.fig` file. File > Export writes PDF, SVG, EPS, PNG and other formats.

## Things worth knowing early

- Most modes have three mouse button functions. See [The mouse](mouse.md) for how to press the middle and right buttons without a three-button mouse.
- Hold Shift to see Shift variants in the mouse indicator.
- Press Escape or the right button to cancel what you are doing in most modes. Escape again clears the selection, and once more leaves the mode. While drawing a line, Backspace removes the last point.
- Coordinates are in 1/1200 inch (Fig units); you work in inches or centimetres as the units box says.
- The figure is never lost when the program stops unexpectedly: see [Files](files.md) (recovery).

## Example drawings

RustFig's repository has a folder of example drawings to open and take apart: <https://github.com/FabianHassler/rustfig/tree/main/examples>.

- **Science gallery** (`examples/gallery/`): sixteen drawings from mathematics and physics (Carnot cycle, double slit, Feynman diagram, RLC circuit, Minkowski diagram and more), with all text in LaTeX. They show how to set up a figure whose labels LaTeX typesets: turn on View > Typeset LaTeX Text to see them typeset on the canvas, and export with File > Export, language "Combined PDF/LaTeX (both parts)".
- **Generated drawings**: a Clifford attractor (250,000 dots), an Apollonian gasket, a Penrose tiling and a Gosper curve.
- **Maps and pictures** imported from SVG: world maps, a topographic map of Greece, landscapes.

Open them with File > Open: the compressed `.fig.gz` files open directly.
