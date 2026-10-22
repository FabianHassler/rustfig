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
- Regions: every region action ("tag region", "delete region", "move region", "update region", "delete in region") takes either a rectangle or a lasso. Click two corners for a rectangle, or press and drag to draw a free-hand loop (the lasso); everything inside is used.

## Shift

Holding Shift changes a button's job in some modes. While Shift is held, the mouse indicator shows the Shift meanings:

- Select: Shift+left adds an object to the selection or removes it.
- While drawing a polyline, polygon, spline or arc: Shift+left removes the last point ("del point"). Shift+middle (not for arcs) finishes the object at the last point placed, without adding the clicked point ("finish here", a RustFig addition).
- Box, Arc-box, Picture: Shift+middle draws a square from its centre.
- Edit attributes: Shift+middle shows the object's comments in the message panel.
- Locate object: in every mode that picks an object (Move, Copy, Delete, Edit, Flip, Rotate, Scale, Convert, Align, Break, Open compound, Join/Split, Chop, Add/Move/Delete point, Update, Arrow, Open/close, Tangent), Shift+left click highlights the object under the pointer (its markers turn filled) without acting on it. Shift+left again highlights the next object under the pointer, front to back; after the last one the pointer position itself is marked, and the next Shift+left starts over. A click without Shift then acts on the highlighted object, not on the front-most one. This is how you reach an object hidden under another one. Changing the mode forgets the highlight. Shift+middle and Shift+right do nothing in these modes (xfig locates with every button; RustFig keeps them free for later use), except that in Edit Shift+middle keeps xfig's own meaning, showing the object's comments.

## Markers and tags

In the editing modes small hollow squares (markers) sit on every object the mode can act on, so you see what to click. Markers stay while a mode waits for its next click (Join's second end, Chop's log, Align's object). Tagged objects show filled markers: objects you tagged with Glue or selected with Select, and the parts of a compound broken with "break and tag". Only Select, Glue and Break tag objects; the other modes act on the object you click and leave the tags as they are, so drawing or editing never tags anything by itself.

## Clicking and dragging

Most placing operations work either way: press, drag and release, or click once to start and click again to finish. A click without movement picks the object under the pointer; the pointer picks within a few screen pixels of the line.

## Wheel and trackpad

Scrolling pans the canvas, Ctrl+scroll or pinching zooms, and holding Space while dragging pans.
