# Text and pictures

## Text

Key `T`. Text objects hold one line each.

- Left, “position cursor”: click where the text should start, then type. Clicking an existing text edits it. Clicking elsewhere finishes the text and starts a new one there.
- Middle, finish text: ends the text being typed. Shift+middle, “paste text” (as in xfig): pastes the system clipboard's text at the cursor of the text being typed. When no text is being typed and the clipboard holds text, middle (shown “paste as text”) pastes it as a new text object at the pointer.
- Right: cancels the text being typed; an edited text keeps its old string.
- Enter finishes the line and starts the next one below it, spaced by the indicator's Text Step. Left, Right, Home, End, Backspace and Delete move and edit.
- Clicking in existing text puts the caret at the clicked character (in the text being typed too). Drag to select; double-click selects a word, triple-click the line; Shift+click extends the selection. Shift+Left, Right, Home and End select with the keys.
<!-- os: mac -->
- Option+Left and Option+Right jump by words, Option+Backspace and Option+Delete delete a word; Cmd+Left and Cmd+Right go to the start and end; Cmd+A selects all, Cmd+C, Cmd+X and Cmd+V copy, cut and paste through the system clipboard. (A lone tap of Option is still xfig's compose key.)
<!-- os: linux windows -->
- Ctrl+Left and Ctrl+Right jump by words, Ctrl+Backspace and Ctrl+Delete delete a word; Ctrl+A selects all, Ctrl+C, Ctrl+X and Ctrl+V copy, cut and paste through the system clipboard.
<!-- os: end -->
- Typing or pasting replaces the selection. Any Unicode text works, including input methods (the composition shows underlined at the caret until you commit it); pasted line breaks are dropped.

### Local fonts

The font chooser (the Text Font button of the indicator, and the Font row of the edit panel) has a third choice beside PostScript fonts and LaTeX fonts: **Local fonts**, every font installed on your computer. Each name is drawn in its own font (only the rows you can see are loaded, so a long list stays fast), the search field narrows the list, and the preview line shows the font at the chosen size. The list is empty in the web build and when RustFig runs with the built-in fonts only.

A Fig file knows fonts only as numbers, so a text in a local font also keeps the nearest Fig font as its number (serif: Times, sans-serif: Helvetica, monospace: Courier, with the weight and slant matched) and the font's name in a comment line of the text, `# rustfig-font: Minion Pro Semibold`. xfig and fig2dev read the file as an ordinary Fig text and ignore the comment; your own comments on the text stay. RustFig draws and measures the text in the installed font, so selecting, editing and the text's box follow it. When a file names a font this computer does not have, the text is drawn in its Fig font and one message on opening lists the missing fonts; saving the file keeps the font name.

Exports (PostScript, PDF, SVG and the others) draw such a text with its Fig font. Embedding the local font in PDF and SVG exports is a follow-up in the fig2dev repository. Scripts use `text.set_local_font` (a font name or `null`, for the selection or `ids`; it also sets the font for new text), `text.local_fonts` (the list) and the `local_font` parameter of `object.set_attributes`.

### Character map

View > Character map... (Ctrl+Shift+M, also while typing) opens a window with the characters of the font of the text being typed. Pick a block (Latin-1, Latin Extended, Greek, punctuation, arrows, mathematical operators and more) or type a character or its code (for example `2211`) in the search field; a click inserts the character at the caret. Only characters the current font can draw are listed (its own face or the fallback face; none shows as an empty box), and each button is drawn in that font (PostScript or LaTeX font as set for the text). Characters are greyed out until you are typing in Text mode.

With the PostScript Symbol font (Greek letters, maths: ∑ ∫ ≤ and so on) or ZapfDingbats the window shows that font's own table. Fig stores such texts as bytes in the font's encoding (∑ is code 0xE5 in Symbol), and RustFig stores exactly that, so the PostScript, PDF, SVG and the other exports draw the right glyph. Typing a character the font does not have is refused. Scripts use the `text.insert_char` command.

The indicator panel sets the font, size, justification, angle and flags (hidden, rigid, special, LaTeX/PostScript fonts) for new text. A special text is for LaTeX (see *LaTeX text, typeset on the canvas* below). Fonts: the PostScript and LaTeX fonts of xfig map to fonts shipped or substituted by RustFig; the picture on the canvas follows fig2dev's font metrics.

## LaTeX text, typeset on the canvas

Set the **special** flag on a text to write LaTeX in it, for example `$\int_0^1 x^2\,dx$`. With a TeX installation (`pdflatex` or `lualatex`, found on the `PATH` or named in Edit > Global settings) the canvas shows it typeset, as it will appear in the document. View > Typeset LaTeX text turns this off. Every text gets `\usepackage{amsmath,amssymb,color,graphicx}`; the **Preamble** field under *LaTeX text* in Edit > Global settings adds more (for example `\usepackage{siunitx}` or your own `\newcommand`s), with **Apply** (commands `view.set_latex_program`, `view.set_latex_preamble`). The picture keeps a little room around TeX's box, so ink that reaches outside the box (the integral sign below the baseline, italic overhangs) is not cut off; clicking and the selection markers use TeX's box. TeX runs in the background, so the source (for example `$\alpha^2$`) shows until the picture is ready, and stays when TeX is missing, off, or reports an error (the message panel names the error). The picture is drawn at the text's anchor, justification and angle; the picture is vector, so it stays sharp at every zoom, and clicking and the selection markers follow its size. Editing still works on the source. The file and the exports are not affected. Without TeX, or in the web build, the source shows as in xfig. The LaTeX-type exports (LaTeX, PSTeX/PDFTeX overlay, pict2e, TikZ, epic, eepic) write it unchanged, without escaping `\`, `$`, `^` or `_`, so LaTeX typesets it; the flag is stored in the `.fig` file.

## Picture

No key (xfig key Ctrl+P). Places an image file in a box.

- Left, “corner point”: drag a box for the picture; then a file dialog asks for the image. The picture is fitted to the box.
- Middle, “center point”: draw the box from its centre (Shift+middle: a square).
- Right, “keep aspect”: draw the box with the right button (click and click again, or drag); the picture then keeps its proportions.

The file name is stored as typed. A relative name is read from the figure's folder, and **Save As** into another folder rewrites relative names so the pictures are still found ([Files](files.md)).

### The picture panel

After the box is drawn and the file chosen, xfig's Edit panel for a picture opens; Edit mode (or Edit attributes) on a picture opens the same panel. Its rows:

- **Picture filename**: edit the name and press Reread to load that file (the box keeps its place). **Browse** picks a file. When the file is missing, a red note says so and Browse becomes **Relink**: pick the file's new place. You can also drop a file from the file manager onto the panel. Apply also takes a changed name.
- **Type**, **Size (pixels)**, **Colours** (counted up to 256) and **Transparent colour** (yes when the picture has transparent pixels). A small **preview** shows the picture.
- **Width** and **Length** resize the box; **Rotation** (0, 90, 180, 270) turns the picture by changing the order of the box corners, as xfig does; **Flipped about diagonal** mirrors it. **h/w ratio** shows the picture's own ratio next to the box's.
- **Shrink to orig** and **Expand to orig** fit the box to the picture's proportions (shrink the longer side, expand the shorter one); **Use original size** gives the box the picture's size at 80 pixels per inch; **Scale by %** sets it to that percentage of the original size. These change the opposite corner in the panel; Apply commits them.
- **Edit Image** opens the file in the system's default application (desktop only; an error appears in the message panel if none starts). Reread afterwards to see the change.
- **XBM pen colour** appears for XBM pictures.

Reread and Relink are undone by Cancel. The same actions are the commands `picture.resize`, `picture.rotate`, `picture.set_file`, `picture.info` and `picture.edit_image`.

File > Import picture does the same without drawing a box first: the picture goes to the centre of the view at its own size. Pictures can be EPS, PDF, PNG, JPEG, GIF, PCX, PPM, TIFF and others; EPS needs ghostscript (optional) to show on the canvas. The picture is referenced by its file name in the figure; keep it next to the figure file.
