# Execution plan

**Status:** 2026-10-13. Current phase: **A (hardening)**, starting with A8 and B0 (fig2dev). Overall parity ≈58 %.

This is the committed work plan. [`gaps.md`](gaps.md) ranks every known
shortfall; this file groups them into phases with task ids, acceptance
criteria and estimates. Commit messages use the task id (`A1: shared undo snapshots`).

## Principles

- Fix foundations before adding features: phase A comes first because its
  problems (undo memory, no CI, unverified platforms) get more expensive
  with every feature added on top.
- Each task is done when its acceptance criteria hold, `cargo xtask ci`
  passes, the app has been run and screenshotted (for UI-visible work), and
  the progress docs are updated (see `CLAUDE.md` › Roadmap and progress docs).
- Estimates are focused hours, not calendar time.

## Phase A: hardening (≈57 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| A1 | **Shared undo snapshots.** Keep top-level objects as `Arc<Object>` (or a persistent vector) so a snapshot copies pointers, not geometry. Today every mutating command deep-copies the document, so 200 undo steps of a 50k-path file can reach gigabytes. | Undo memory for 200 single-object edits on the 50k-path document stays under 50 MB; `rustfig-cli perf` adds an "edit + undo" row within budget. | 8 |
| A2 | **CI pipeline** (GitHub Actions): `cargo xtask ci` on macOS, Windows and Linux, with the wasm target installed and `RUSTFIG_REQUIRE_WASM=1`. | Green on all three OSes; wasm check runs, not skipped. | 6 |
| A3 | **Web build compiles.** Fix whatever `cargo check --target wasm32-unknown-unknown -p rustfig` reports (std::fs/threads/time behind cfg). | `cargo xtask wasm` passes in CI. | 6 |
| A4 | **UI smoke tests** with `egui_kittest`: open the app headless, click tools, draw a box, open each dialog. | Tests run in `cargo test`, no window needed. | 8 |
| A5 | **Export polish:** a default border so arrow tips and stroke widths aren't clipped; SVG/PDF/EPS emit true curves for ellipses, arcs and splines (Béziers) instead of dense polylines. | Sample file exports unclipped; SVG of the sample shrinks by more than 50 %. | 8 |
| A6 | **Crash safety:** autosave to a recovery file every N edits and on panic (xfig writes `SAVE.fig`); offer recovery on start. | Kill the app mid-edit; the next start offers to restore. | 5 |
| A7 | **Render cache for large drawings:** reuse the built scene between frames and rebuild only on revision change; spatial index for culling. | 50k-path pan render < 150 ms (now ≈330 ms). | 4 |
| A8 | **fig2dev reference tests.** Render test figures through fig2dev and RustFig and compare them (design below). | `cargo test` runs the comparison when fig2dev is installed and skips otherwise; each fixture has a recorded score; the diff images are written for review. | 12 |

## Phase B: exports for LaTeX users, M4 (≈88 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| B0 | **fig2dev export backend** (desktop only; design below). Covers the LaTeX family, CGM, EMF, DXF, MetaPost, GIF/JPEG/TIFF and more right away. B1–B6 then replace it where fidelity or the web build need native code. | With fig2dev installed, `file.export` writes every fig2dev language; without it, those formats show as unavailable and nothing breaks. | 8 |
| B1 | **TikZ export** (`.tikz`/`.tex`): paths, fills, patterns, arrows, dashes, text with special flag passed through raw. | Compiles with pdflatex; a visual diff against the PDF export is close. | 16 |
| B2 | **PDF/EPS + LaTeX overlay** (fig2dev's pdftex_t/pstex_t): graphics without special text, plus a `.tex` file placing the text. | The combined output typesets in LaTeX with fonts from the document. | 12 |
| B3 | **PSTricks and pict2e exports.** | Both compile; a test checks the structure. | 14 |
| B4 | **PostScript with page layout:** orientation, centring, paper size, magnification, multi-page tiling (the Fig header's "Multiple"). | Output matches the header settings; ghostscript renders it (manual check). | 12 |
| B5 | **Printing**: native print via the OS (PDF to the system print dialog), command `file.print`. | Prints on macOS; graceful error elsewhere. | 10 |
| B6 | **Bitmaps:** GIF (with the Fig transparent colour), JPEG (quality), TIFF, PPM. | Round trip through the `image` crate in tests. | 6 |
| B7 | **Export dialog update** for all new formats, remembering the last choice. | Screenshot review. | 4 |
| B8 | Accurate text widths in PDF/EPS: use the exported font's metrics (craft-fonts faces matching the standard 35) for justification. | Centred text in the PDF centres within 1 % of the line width. | 6 |

## Phase C: web, M5 (≈30 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| C1 | `trunk` packaging (`web/index.html`), `cargo xtask web` builds `dist/`. | Opens in Chrome, Firefox, Safari. | 6 |
| C2 | Browser file I/O: open via file picker or drag-and-drop, save/export via download (async `rfd`). | Open, edit, save and export round trip in the browser. | 10 |
| C3 | Autosave to browser storage; restore on reload. | Reload keeps the drawing. | 4 |
| C4 | Touch and trackpad: pinch zoom, two-finger pan, long-press as right click. | Usable on an iPad (manual check). | 6 |
| C5 | Publish a demo build (**user decision**: where and under which name). | — | 4 |

## Phase D: remaining editing modes, M6 (≈110 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| D1 | **Open compound**: edit members in place, with the rest dimmed; close returns to the top level. | Move a member without breaking the compound; undo works. | 16 |
| D2 | **Join/split** lines and splines at shared end points. | Tests for joining two polylines and splitting one. | 10 |
| D3 | **Chop** (cut objects at intersections). | Tests with line/line, line/arc and line/ellipse. | 14 |
| D4 | **Tangent/normal** tool for circles, ellipses, arcs and splines. | Draws the correct line at the clicked point. | 8 |
| D5 | **Measure modes**: angle, length, area tools showing results in the status bar. | Matches `object.measure`. | 6 |
| D6 | **Library objects**: browse a directory of .fig files with thumbnails, place them as compounds; copy a selection to a library. | Place an object from a sample library; undo removes it. | 30 |
| D7 | **Point positioning modes**: latex lines, manhattan, mountain, 45° constraint with Shift. | Tests per mode. | 10 |
| D8 | **Linked move**: moving a box drags the attached line end points (xfig's smart links). | Test with a box and two attached lines. | 10 |
| D9 | Arrowhead types 10–14 with exact shapes; dash-dot tick styles. | Visual review against the xfig manual. | 6 |

## Phase E: UI completeness, M7 (≈75 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| E1 | **Settings persistence** (eframe storage on native and web): indicator, grid, units, panel visibility, recent files. | Restart keeps settings; File menu lists recent files. | 8 |
| E2 | **Rulers** with cursor tracking, units, and drag to pan. | Screenshot review. | 8 |
| E3 | **Zoom box** and zoom tool. | Drag a box to zoom into it. | 4 |
| E4 | **Colour editor**: create and edit user colours (RGB/HSV), remove unused ones. | New colours save to the Fig file. | 10 |
| E5 | **Character map** inserting into the text being typed (Symbol and Dingbats included). | Insert ∑ with the Symbol font. | 8 |
| E6 | **Style manager**: named attribute sets applied to the indicator. | Save, apply, delete a style; persists. | 10 |
| E7 | **System clipboard**: copy objects as Fig text and SVG, paste Fig text and images. | Copy between two RustFig windows; paste an image from a browser. | 10 |
| E8 | **Keyboard and mouse help**: a mouse-function hint bar like xfig's, and a shortcut sheet. | Screenshot review. | 5 |
| E9 | **Dark/light theme switch** and UI scale setting. | Screenshot review. | 4 |
| E10 | Snap to object points (end points, centres, intersections) besides the grid. | Tests. | 8 |

## Phase F: text, fonts and languages (≈60 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| F1 | **craft-fonts integration**: map the 35 PostScript families to craft-fonts faces (URW base35 equivalents if present); test glyph rendering when `CRAFT_FONTS` isn't empty. | Times/Helvetica/Courier render with the right faces; tests skip when empty. | 10 |
| F2 | **LaTeX special text** on screen (shown as raw source, marked) and in exports (B1/B2). | Special flag round-trips; exports pass it raw. | 6 |
| F3 | Compose-key / dead-key input in the text tool. | Type é with a compose sequence. | 6 |
| F4 | **Localization framework** (Fluent or similar) with English and German. | Switching the language translates menus and panels. | 20 |
| F5 | Spell check of text objects (Hunspell dictionaries, optional). | Flags misspelt texts. | 12 |
| F6 | Right-to-left and complex-script shaping (rustybuzz). | Arabic sample shapes correctly. | 6 |

## Phase G: file formats and Beta (≈60 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| G1 | Read Fig 1.x/2.x. | Sample files from the format docs load. | 24 |
| G2 | Picture objects: XPM, XBM, PCX via code; PDF via a pure-Rust renderer; EPS preview (TIFF/WMF preview section) or a labelled box. Adding a PDF renderer dependency is a **user decision**. | Each format shows in the canvas and exports. | 20 |
| G3 | Accessibility: AccessKit labels for panels and dialogs; object list for screen readers. | VoiceOver reads the tool panel. | 10 |
| G4 | **Beta review**: re-score parity, aim for ≥ 85 %; fix the top regressions. | Updated `target-app-parity.md`. | 6 |

## fig2dev: design notes for A8 and B0

fig2dev 3.2.9a is installed here (`/opt/homebrew/bin/fig2dev`). It is
xfig's own converter: xfig's Export menu saves a `.fig` file and runs it.
RustFig keeps its native PDF, SVG, EPS and PNG exporters (needed for the
web build, WYSIWYG with the canvas, and machines without fig2dev). fig2dev
is an optional extra, used in two ways.

### B0: export backend

- **Module** `crates/engine/src/fig2dev.rs`, compiled only on native targets (`cfg(not(target_arch = "wasm32"))`). No UI code; the engine owns process handling, so the CLI, MCP and control channel get it too.
- **Finding the binary:** `RUSTFIG_FIG2DEV` (explicit path) → `PATH` → `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin` (GUI apps on macOS don't inherit the shell's `PATH`). Cache the result and the version (`fig2dev -V`).
- **Languages:** an allowlist matching 3.2.9a (`box cgm dxf eepic eepicemu emf epic eps gbx ge gif ibmgl jpeg latex map mf mp pcx pdf pdftex pdftex_t pic pict2e pictex png ppm ps pstex pstex_t pstricks ptk shape sld svg textyl tiff tikz tk tpic xbm xpm`). Never pass anything that isn't on it. Arguments go to `std::process::Command` as separate items, never through a shell.
- **Running it:** write the document with `write_fig` to a temporary file. Run `fig2dev -L <lang> [-m <mag/100>] [-b <border>] [-g <background>] in.fig out` with the working directory set to the document's directory, so relative picture paths resolve. Kill it after a timeout (60 s). Return its stderr as the error message on failure. Remove temporary files afterwards.
- **Options mapping:** magnification → `-m`; margin → `-b` (pixels for bitmaps, points for vector drivers, converted from Fig units); background → `-g` for bitmaps; driver-specific extras (TikZ `-P` stand-alone page, pdftex `-p` include name) via an `options` object passed through only for known flags.
- **Commands:**
  - `file.export` gains `engine: "native" | "fig2dev" | "auto"` (default `auto`: native for svg/eps/pdf/png, fig2dev for the rest).
  - `file.export_data` works for fig2dev languages too (text formats as `data`, binary as `base64`).
  - New `file.export_formats`: available formats with their engine and extension, and whether fig2dev was found.
  - New `file.export_latex_overlay` {path, kind: pdftex|pstex}: writes the graphics file plus the `_t` LaTeX file referencing it (two fig2dev runs).
- **Formats from file names:** `.tikz`, `.pstricks`, `.cgm`, `.emf`, `.dxf`, `.mp`, `.mf`, `.gif`, `.jpg`/`.jpeg`, `.tif`/`.tiff`, `.ppm`, `.pcx`, `.xbm`, `.xpm`, `.ps`, `.pic`, `.tk`. A bare `.tex` is ambiguous, so it needs an explicit `format`.
- **UI:** the export dialog lists native formats first, then fig2dev formats (greyed out with a hint when fig2dev is missing), and remembers the last choice.
- **CLI:** `rustfig-cli convert IN OUT [--format LANG] [--engine native|fig2dev]`, and `rustfig-cli formats`.
- **Tests:** unit tests for argument building and the allowlist; an integration test that exports every language from the sample file and checks a non-empty result, skipping when fig2dev is absent. Extend `command_sweep.rs` with hostile `format`/`options` values.
- **Docs:** `file-format-parity.md` rows for each language (engine: fig2dev); `NOTICE` says fig2dev is an optional external program, not bundled; README and `development.md` explain `RUSTFIG_FIG2DEV`.

### A8: reference tests

- **Fixtures** `crates/engine/tests/fixtures/*.fig`, written for RustFig (test data, not copied from xfig). One file per feature: line widths and dash styles, caps and joins, every arrowhead type and size, shades and tints, each fill pattern, the three spline kinds with mixed shape factors, arcs in both directions with arrows, rotated ellipses, text with each justification, angle and several fonts, compounds and depths, user colours, metric units.
- **Rendering:** fig2dev `-L png` at a fixed magnification with `-S 4` smoothing and a white background; RustFig through `file.export_data` (png) at the same resolution and background.
- **Comparing:** trim both images to their non-white bounding box, scale them to a common size, convert to greyscale, blur slightly (to tolerate anti-aliasing and 1-pixel offsets) and compute the mean absolute difference plus the share of strongly differing pixels.
- **Baselines:** record each fixture's score in `fixtures/scores.json`. The test fails when a score gets worse than its baseline by more than a margin, so it catches regressions without requiring pixel equality. An `UPDATE_REFERENCE=1` run rewrites the baselines after an intended change.
- **Review output:** write `fig2dev.png`, `rustfig.png` and `diff.png` per fixture to `target/fig2dev-reference/` for screenshot review; `rustfig-cli compare FILE` does the same for any file.
- **Known differences to expect** (document rather than chase): fig2dev's bitmap line widths use 1/80 inch unless `-F`, font substitution (it uses real PostScript fonts through ghostscript), and pattern phase. Fixtures with the biggest differences become tasks (D9, F1 and new ones).
- **CI:** install fig2dev on the Linux runner so the comparison runs there; it skips elsewhere.

## Order and milestones

1. A8 and B0 first (fig2dev reference tests, then the fig2dev export backend), then the rest of phase A, then B1, B2, C1–C3: the first release candidate for LaTeX users, on desktop and web.
2. D1, D6, E1, E2, E7: the features xfig users miss most day to day.
3. The rest of B, D and E, then F and G toward Beta (parity ≥ 85 %, about 460 h in total).

## Decisions that are the user's

- Clean-room policy for `old/` (whether agents may read the xfig C sources, or only its documentation).
- New heavy dependencies (PDF rendering, spell checking, localization framework).
- fig2dev stays an optional external program (agreed 2026-10-13): never bundled, never required for the native formats or the web build.
- Publishing (demo site, crate names, releases) and the product name/branding.
- Licence changes and any asset of unclear provenance.

## Revision history

| Date | Note |
|------|------|
| 2026-10-13 | First plan, after M0–M3. |
| 2026-10-13 | Added A8 (fig2dev reference tests) and B0 (fig2dev export backend) with design notes. |
