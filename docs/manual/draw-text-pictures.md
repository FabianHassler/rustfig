# Text and pictures

## Text

Key `T`. Text objects hold one line each.

- Left, “position cursor”: click where the text should start, then type. Clicking an existing text edits it. Clicking elsewhere finishes the text and starts a new one there.
- Middle, finish text: ends the text being typed. Shift+middle, “paste text” (as in xfig): pastes the system clipboard's text at the cursor of the text being typed. When no text is being typed and the clipboard holds text, middle (shown “paste as text”) pastes it as a new text object at the pointer.
- Right: cancels the text being typed; an edited text keeps its old string.
- Enter finishes the line and starts the next one below it, spaced by the indicator's Text Step. Left, Right, Home, End, Backspace and Delete move and edit.
- xfig's Control keys edit the line (`d_text.c`; on macOS the Control key itself, on Linux and Windows those that do not clash with Select all, Cut and Paste): Ctrl-A start of the text (macOS only), Ctrl-E end, Ctrl-B back, Ctrl-F forward, Ctrl-D delete the character at the caret, Ctrl-H backspace, Ctrl-K delete to the end of the text, Ctrl-X delete from the start to the caret (macOS only; Cut elsewhere), Ctrl-W delete the selection.
- **Superscripts and subscripts**: Ctrl-^ (Ctrl+Shift+6) ends the text so far and starts the next piece at the caret, a quarter of a line higher and at three quarters of the size; Ctrl-_ (Ctrl+Shift+-) does the same lower. The opposite key steps back out again, up to four levels deep; Enter starts the next line below the first piece at the original size. Each piece is a text object of its own, as in xfig.
- Clicking in existing text puts the caret at the clicked character (in the text being typed too). Drag to select; double-click selects a word, triple-click the line; Shift+click extends the selection. Shift+Left, Right, Home and End select with the keys.
<!-- os: mac -->
- Dead keys and input methods work: Option+E then E types é; the pending accent is shown at the caret and replaced by the finished character.
<!-- os: linux windows -->
- Dead keys and input methods work: the compose or dead keys type é and the like; the pending accent is shown at the caret and replaced by the finished character.
<!-- os: end -->
<!-- os: mac -->
- Option+Left and Option+Right jump by words, Option+Backspace and Option+Delete delete a word; Cmd+Left and Cmd+Right go to the start and end; Cmd+A selects all, Cmd+C, Cmd+X and Cmd+V copy, cut and paste through the system clipboard. (A lone tap of Option is still xfig's compose key.)
<!-- os: linux windows -->
- Ctrl+Left and Ctrl+Right jump by words, Ctrl+Backspace and Ctrl+Delete delete a word; Ctrl+A selects all, Ctrl+C, Ctrl+X and Ctrl+V copy, cut and paste through the system clipboard.
<!-- os: end -->
- Typing or pasting replaces the selection. Any Unicode text works, including input methods (the composition shows underlined at the caret until you commit it); pasted line breaks are dropped.

### Local fonts

The font chooser (the Text Font button of the indicator, and the Font row of the edit panel) has a third choice beside PostScript fonts and LaTeX fonts: **Local fonts**, every font installed on your computer that has the Latin letters. Hidden macOS system fonts (names starting with a dot, such as `.SF NS`) are left out, and so are fonts without Latin letters (Arabic, Hebrew, Indic and other scripts, such as Waseem): RustFig lays text out left to right without joining letters, so text in those scripts would not come out right. Each name is drawn in its own font (only the rows you can see are loaded, so a long list stays fast), the search field narrows the list, and the preview line shows the font at the chosen size. The list is empty in the web build and when RustFig runs with the built-in fonts only.

A Fig file knows fonts only as numbers, so a text in a local font also keeps the nearest Fig font as its number (serif: Times, sans-serif: Helvetica, monospace: Courier, with the weight and slant matched) and the font's name in a comment line of the text, `# rustfig-font: Minion Pro Semibold`. xfig and fig2dev read the file as an ordinary Fig text and ignore the comment; your own comments on the text stay. RustFig draws and measures the text in the installed font, so selecting, editing and the text's box follow it. When a file names a font this computer does not have, the text is drawn in its Fig font and one message on opening lists the missing fonts; saving the file keeps the font name.

Exports: PDF, EPS and PostScript embed the local font (a subset of the glyphs used, as a TrueType/Type 42 font), PNG and the other bitmaps draw it, and SVG names it with its Fig font as the fallback (the viewer must have the font installed). The TeX languages (LaTeX, PSTricks, TikZ, pict2e and the others) and xfig or the C fig2dev reading the file use the Fig font.

### Character map

View > Character map... (Ctrl+Shift+M, also while typing) opens a window with the characters of the font of the text being typed. Pick a block (Latin-1, Latin Extended, Greek, punctuation, arrows, mathematical operators and more) or type a character or its code (for example `2211`) in the search field; a click inserts the character at the caret. Only characters the current font can draw are listed (its own face or the fallback face; none shows as an empty box), and each button is drawn in that font (PostScript or LaTeX font as set for the text). Characters are grayed out until you are typing in Text mode.

With the PostScript Symbol font (Greek letters, math: ∑ ∫ ≤ and so on) or ZapfDingbats the window shows that font's own table. Fig stores such texts as bytes in the font's encoding (∑ is code 0xE5 in Symbol), and RustFig stores exactly that, so the PostScript, PDF, SVG and the other exports draw the right glyph. Typing a character the font does not have is refused.

The indicator panel sets the font, size, justification, angle and flags (hidden, rigid, TeX, LaTeX/PostScript fonts) for new text. A text with the TeX flag holds LaTeX (see *LaTeX text, typeset on the canvas* below). Fonts: the PostScript and LaTeX fonts of xfig map to fonts shipped or substituted by RustFig; the picture on the canvas follows fig2dev's font metrics.

## Search/Replace and spell check

Edit > Search/Replace... (`Ctrl+F`, xfig's Search and Replace panel) finds texts, also inside compounds. Type the string in **Search for** and press Return or **Search**: the matching texts are selected and listed as "[x=… y=…] text" with their position, an empty string matches every text. **Case sensitive** is on at the start, **Whole word** matches only where the match is not part of a longer word (letters, digits and `_` count as word characters, so "cat" finds "cat." and "a cat" but not "concatenate" or "cat_s"). **Replace with**, Return in that field or **Replace** changes every match in the found texts (replacing with nothing asks for a second click, as in xfig), and **Update settings** gives the found texts the attribute panel's font, size, color and flags. **Next** and **Previous** (added) select one matching object at a time, from the selected one on and wrapping round at the end, and scroll the drawing to show it; the message line says "Match 2 of 5". Clicking a line of the result list selects that text and scrolls to it. Regular expressions are not supported yet.

Edit > Spell check writes the text of every text object to a temporary file and runs the spell checker of Global settings (`aspell list < %f` by default; if `aspell` is missing, `hunspell -l %f`; `%f` stands for the file). The words it prints open a "Misspelled words" window with a message line ("3 misspelled words found"). Click a word to put it in the **Correction** field, edit it and press **Correct** (or Return): the word is replaced in every text, as Search/Replace would, and leaves the list. **Ignore** drops a word from the list without changing anything, **Recheck** runs the checker again, **Dismiss** (or Escape) closes the window. At most 200 words are listed, and the checker is stopped after 30 seconds. If no spell checker is installed the message says so; set the command in Global settings. Spell check is not available in the web version.

## Find properties

Tools > Find Properties... (added) starts Find mode, a mode like xfig's editing modes but without a button in the mode panel. While it is on, its window is open, the attribute panel shows what it shows in Update mode (every attribute with its update box, the Update Control box and the line "Update will change: ..."), and the matches are outlined on the canvas. Choosing another mode, pressing Escape or closing the window ends the mode and forgets the matches.

The matches start empty. Two things change them. The window has one row per property, each with a tick box: a ticked property must match, an unticked one is ignored, and with nothing ticked (and nothing clicked) the window says "Tick a property or click objects". The values are set with the controls of the Edit panel: **Object kind** (line, box, polygon, spline, arc, ellipse, text, picture, compound), **Line width** (number pop-up), **Line style** (picture menu), **Pen color** and **Fill color** (color menu), **Fill style** (none, filled, pattern), **Arrows** (with or without), **Depth** from and to (number pop-ups), **Font**, **Font size** and **Text contains** (part of the string, case does not matter). An unticked control is grayed and keeps its last value. Each change replaces the matches. By default only top-level objects are tested; a compound matches by its kind and depth. **Also inside compounds** also looks at the members, at any depth of nesting; the compound then is the match. And the mouse corrects the result by hand: the left button on an object adds it to the matches or, when it is one, removes it (indicator: “add/remove object”). The middle and right buttons do nothing in this mode. Shift+left locates an object under others, as in the other editing modes.

The window says how many objects were found ("3 objects found") and, under **Show list**, lists them (a click on a line scrolls to the object). The buttons act on all matches at once, each as one undo step: **Update to indicator** gives them the attribute panel's settings, by the update boxes you see (as Update mode does), **Set depth** gives them the depth on its number button, **Glue** makes one compound of them (which then is the only match), and **Delete** removes them (the matches are then empty).

Scripts: see [Automation](automation.md).

## LaTeX text, typeset on the canvas

Set the **TeX** flag (the Fig format calls it the *special* flag, as do xfig's manual and its `-specialtext` option) on a text to write LaTeX in it, for example `$\int_0^1 x^2\,dx$`. With a TeX installation (`pdflatex` or `lualatex`, found on the `PATH` or named in Edit > Global settings) the canvas shows it typeset, as it will appear in the document. View > Typeset LaTeX text turns this off. A figure carries its own LaTeX preamble, saved in its `.fig` file, and its texts get no other packages besides `\usepackage{color,graphicx}` (what the PDFTeX/PSTeX overlay needs from your document), so the figure typesets the same on any computer and with `rustfig-cli convert`. Put what its texts need there, for example `\usepackage{amsmath}` for `\text` or `\dfrac`, `\usepackage{siunitx}` or your own `\newcommand`s. Edit it in **Edit > Page setup** (the field *LaTeX preamble of this figure*, used when you leave it or close the window) or in the **Whole Figure** dialog (Edit mode, middle button on the canvas). The **Preamble** field under *LaTeX text* in Edit > Global settings is the preamble new figures start with; it changes no figure you already have, except the open one when you click **Reapply to figure** (undoable). A file without a preamble, such as one from xfig, gets none. The preamble is stored only when the figure has TeX texts, as comment lines `# RustFigLaTeXPreamble …`, which xfig keeps and ignores; fig2dev uses them only where its output is a complete document or a program C fig2dev does not write (TikZ, pict2e and PSTricks with the full-document option `-P`, and Asymptote, as `texpreamble`), and elsewhere prints them as comments, as C fig2dev does; merging a figure or placing a library object adds the preamble lines it brings that this figure lacks. When TeX reports an unknown command (`Undefined control sequence`), the message suggests adding the package to the figure's preamble. The picture keeps a little room around TeX's box, so ink that reaches outside the box (the integral sign below the baseline, italic overhangs) is not cut off; clicking and the selection markers use TeX's box. TeX runs in the background, so the source (for example `$\alpha^2$`) shows until the picture is ready, and stays when TeX is missing, off, or reports an error (the message panel names the error). Objects you place, move or copy show TeX's picture while they follow the mouse; a text you are typing shows its source. The picture is drawn at the text's anchor, justification and angle; the picture is vector, so it stays sharp at every zoom, and clicking and the selection markers follow its size. Editing still works on the source. The file and the exports are not affected. Without TeX, or in the web build, the source shows as in xfig. The LaTeX-type exports (LaTeX, PSTeX/PDFTeX overlay, pict2e, TikZ, epic, eepic) write it unchanged, without escaping `\`, `$`, `^` or `_`, so LaTeX typesets it; the flag is stored in the `.fig` file.

## Picture

Key Ctrl+P with the Control key itself (xfig's): always on a Mac (⌃ P); on Windows and Linux only with xfig's keys on, since Ctrl+P is Print there. Places an image file in a box.

- Left, “corner point”: drag a box for the picture; then a file dialog asks for the image. The picture is fitted to the box.
- Middle, “center point”: draw the box from its center (Shift+middle: a square).
- Right, “keep aspect”: draw the box with the right button (click and click again, or drag); the picture then keeps its proportions.

In the web version the file dialog is the browser's picker and the picture lives in memory under its file name until the page is closed (see [Files](files.md)).

The file name is stored as typed. A relative name is read from the figure's folder, and **Save As** into another folder rewrites relative names so the pictures are still found ([Files](files.md)).

### The picture panel

After the box is drawn and the file chosen, xfig's Edit panel for a picture opens; Edit mode (or Edit attributes) on a picture opens the same panel. Its rows:

- **Picture filename**: edit the name and press Reread to load that file (the box keeps its place). **Browse** picks a file. When the file is missing, a red note says so and Browse becomes **Relink**: pick the file's new place. You can also drop a file from the file manager onto the panel. Done also takes a changed name.
- **Type**, **Size (pixels)**, **Colors** (counted up to 256) and **Transparent color** (yes when the picture has transparent pixels). A small **preview** shows the picture.
- **Width** and **Length** resize the box; **Rotation** (0, 90, 180, 270) turns the picture by changing the order of the box corners, as xfig does; **Flipped about diagonal** mirrors it. **h/w ratio** shows the picture's own ratio next to the box's.
- **Shrink to orig** and **Expand to orig** fit the box to the picture's proportions (shrink the longer side, expand the shorter one); **Use original size** gives the box the picture's size at 80 pixels per inch; **Scale by %** sets it to that percentage of the original size. These change the opposite corner in the panel and on the canvas at once.
- **Edit Image** opens the file in the system's default application (desktop only; an error appears in the message panel if none starts). Reread afterwards to see the change.
- **Import as objects** (PDF pictures only, found by the file's content; an EPS picture cannot be imported, and the button stays gray) replaces the picture by the objects of the PDF's first page, as one compound that fills the picture's box (flipped pictures are transposed as the canvas draws them) at the picture's depth. It uses the same importer as File > Open of a PDF (see [Files](files.md)), so text becomes text and lines become lines; whatever the importer drops or approximates is listed in the message panel. The panel's changes are kept first (one undo step), the import is a second one. When the PDF carries an embedded RustFig figure, the page's vectors are imported anyway.
- **XBM pen color** appears for XBM pictures.

Reread and Relink are undone by Cancel.

Tools > Import Picture... does the same without drawing a box first: the picture goes to the center of the view at its own size. Pictures can be EPS, PDF, PNG, JPEG, GIF, PCX, PPM, TIFF and others; EPS needs ghostscript (optional) to show on the canvas. The picture is referenced by its file name in the figure; keep it next to the figure file.

## Boxed text and the vertical anchor

Two additions to the Text mode indicator (neither is in xfig; the result is plain Fig objects xfig opens and draws the same):

- **Boxed** in the Text Flags pop-up (with its **Margin**, in Fig units): a new text gets a box one depth behind it, in the indicator's fill color (full fill when the indicator has no fill), glued with the text into a compound. The box has the indicator's pen color and line width (set Line Width to 0 for no border). A rotated text gets a rotated box (a polygon). A LaTeX text is boxed by the size TeX gives it when the typeset picture is known, else by its stored size.
- **Anchor** in the Text Just pop-up: **top**, **middle** or **baseline**. Baseline is xfig's behavior (the clicked point is the baseline at the justification); top and middle make the point the top or the vertical middle of the text, which is handy to put a label at a given spot. It applies when the text is placed (the text lands where the anchor says when you finish typing).

