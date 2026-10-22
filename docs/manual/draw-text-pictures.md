# Text and pictures

## Text

Key `T`. Text objects hold one line each.

- Left, “position cursor”: click where the text should start, then type. Clicking an existing text edits it. Clicking elsewhere finishes the text and starts a new one there.
- Middle, finish text: ends the text being typed. When no text is being typed and the clipboard holds text, middle (shown “paste as text”) pastes it as a new text object at the pointer.
- Right: cancels the text being typed; an edited text keeps its old string.
- Enter finishes the line and starts the next one below it, spaced by the indicator's Text Step. Left, Right, Home, End, Backspace and Delete move and edit.

The indicator panel sets the font, size, justification, angle and flags (hidden, rigid, special, LaTeX/PostScript fonts) for new text. Fonts: the PostScript and LaTeX fonts of xfig map to fonts shipped or substituted by RustFig; the picture on the canvas follows fig2dev's font metrics.

## Picture

No key (xfig key Ctrl+P). Places an image file in a box.

- Left, “corner point”: drag a box for the picture; then a file dialog asks for the image. The picture is fitted to the box.
- Middle, “center point”: draw the box from its centre (Shift+middle: a square).
- Right, “keep aspect”: draw the box with the right button (click and click again, or drag); the picture then keeps its proportions.

File > Import picture does the same without drawing a box first: the picture goes to the centre of the view at its own size. Pictures can be EPS, PDF, PNG, JPEG, GIF, PCX, PPM, TIFF and others; EPS needs ghostscript (optional) to show on the canvas. The picture is referenced by its file name in the figure; keep it next to the figure file.
