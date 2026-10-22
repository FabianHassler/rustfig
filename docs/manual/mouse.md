# The mouse

xfig is drawn with three mouse buttons, and almost every mode gives each button a different job (left: the main action, middle: a variation, right: cancel or another variation). RustFig keeps that exactly. The mouse indicator in the top strip always shows what the three buttons do right now.

## Getting three buttons

| Computer | Left | Middle | Right |
|---|---|---|---|
| Any, with a three-button mouse | left button | middle button (wheel click) | right button |
| macOS, trackpad or one-button mouse | click | Option+click | Ctrl+click (or a two-finger click) |
| Linux and Windows, two-button mouse | left button | left and right buttons together (a chord) | right button |
| Any, keyboard substitute | click | Alt+click | Ctrl+click |

Alt+click and Ctrl+click work on every platform; on macOS Option is Alt. The chord on Linux and Windows works like xfig on a two-button mouse: press both buttons within a moment of each other.

## What the buttons do

The three functions of each mode are listed in the mode's section ([shapes](draw-shapes.md), [splines](draw-splines.md), [select and move](select-move.md) and the other editing topics). Typical patterns:

- Drawing: left adds the next point or starts the shape, middle is a variation (freehand, from the centre) or the final point, right cancels.
- Editing: left acts on the object clicked, middle often acts on a copy or restricts a move to horizontal or vertical, right is a variation or region action.
- Regions: where a mode says "(drag: lasso)" you can click two corners for a rectangle, or press and drag to draw a free-hand loop; everything inside is used.

## Shift

Holding Shift changes a button's job in some modes. While Shift is held, the mouse indicator shows the Shift meanings:

- Select: Shift+left adds an object to the selection or removes it.
- While drawing a polyline, polygon, spline or arc: Shift+left removes the last point ("del point").
- Box, Arc-box, Picture: Shift+middle draws a square from its centre.
- Edit attributes: Shift+middle shows the object's comments in the message panel.
- Rotate modes: Shift reverses the direction (the rotate buttons turn the other way and copy the other way).
- Chop: Shift+left trims the clicked piece, Shift+middle splits at the point, Shift+right extends a line end ([Chop](chop.md)).

## Clicking and dragging

Most placing operations work either way: press, drag and release, or click once to start and click again to finish. A click without movement picks the object under the pointer; the pointer picks within a few screen pixels of the line.

## Wheel and trackpad

Scrolling pans the canvas, Ctrl+scroll or pinching zooms, and holding Space while dragging pans.
