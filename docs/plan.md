# Execution plan

**Status:** 2026-10-14. Phase W (xfig window parity) done; next A8 and B0 (fig2dev), then the rest of phase A. Overall parity ≈60 %.

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
- Port from xfig's and fig2dev's sources (`old/`, xfig licence) where they
  cover the task: [`source-map.md`](source-map.md) names the C file for each
  area. A task that touches an "own" row reviews it against that file;
  accepted behaviour differences are recorded in the map.

## Phase S: source review (≈20 h)

The source map compares every xfig and fig2dev file with RustFig. These
tasks fix what it found; the per-area reviews ride along with the W, B and D
tasks that touch each area.

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| S0 | ✅ 2026-10-14. **Source map** `docs/source-map.md`: every xfig and fig2dev C file with its RustFig module, status and task. | Every C file listed. | 4 |
| S1 | ✅ 2026-10-14 (`Object::xfig_type_rank`, `Document::for_each_leaf_xfig_order`, `xfig_order`; hit testing follows the same order). **Draw order as xfig:** within a depth, arcs, compounds (recursively), ellipses, lines, splines, texts (`u_redraw.c`); gray depths drawn behind all active ones; `.fig` written in xfig's type order (`f_save.c`). | Render tests for type order, compound order and gray layers; a saved file lists objects in xfig's order. | 4 |
| S2 | ✅ 2026-10-14 (also: xfig's labels word for word; ellipses finish with either button; text: middle finishes, right cancels). **Mouse indicator as xfig** (`w_mousefun.c`): left label left of the mouse, middle above, right label right; xfig's capitalisation. | Screenshot next to the manual's image. | 2 |
| S3 | **Startup options and resources** (`main.c`, `resources.c`, `xfigmain.c`): the command-line options users pass (`-metric`, `-specialtext`, `-latexfonts`, `-startfontsize`, `-exportLanguage`, `-startgridmode`, `-startposnmode`, `-zoom`, arrow defaults, …) set the same defaults in RustFig; saved with E1 settings. | `rustfig -metric -specialtext …` starts with the same indicator settings as xfig; tests per option. | 10 |

## Phase A: hardening (≈84 h)

Checked against the code on 2026-10-13. Measured on a document of 50 000
polylines and 2 000 texts (release build): one undo snapshot is about
14.5 MB and takes 9 ms to copy; a one-object `transform.move` costs 5.9 ms,
most of it spent copying the document and re-measuring every text.

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| A1 | **Shared undo snapshots.** `Engine::execute` deep-copies the whole document before every mutating command, and `History` keeps up to 200 copies: about 2.9 GB for the measured document. Store top-level objects as `Arc<Object>` so a snapshot copies pointers, and mutate through `Arc::make_mut` only on the objects a command touches (about 15 `doc_mut().objects.iter_mut()` call sites in `cmd/*`). Cap the history by memory as well as by step count. | 200 single-object edits on the 50k-path document add < 50 MB; snapshot cost < 1 ms; `rustfig-cli perf` gains an "edit + undo" row within budget. | 10 |
| A2 | **CI pipeline** (GitHub Actions; none exists yet). **Every push and pull request:** `cargo xtask ci` on Linux, with the wasm target installed, `RUSTFIG_REQUIRE_WASM=1`, fig2dev installed for A8, and a cargo cache. **Releases only** (tags `v*`, plus manual `workflow_dispatch`): the same on Windows and macOS, followed by release builds attached to the GitHub release. Linux needs system packages for eframe/glow and for `rfd` (GTK 3 by default; switching `rfd` to its xdg-portal backend avoids GTK). | Linux job green on every push with the wasm check running, not skipped; pushing a `v*` tag runs all three OSes and produces release binaries. | 8 |
| A3 | 🟡 2026-10-14: compiles locally (`cargo xtask ci` runs the wasm check with rustup's stable toolchain); the CI step remains. **Web build compiles.** Code review found no blockers so far: native-only code (rfd, control channel, `Instant`) is cfg-gated and `getrandom` isn't in the wasm dependency tree. Still to confirm: the eframe `wayland`/`x11` features on wasm, `eframe::wasm_bindgen` in `apps/rustfig`, and `std::fs` calls (they compile, then fail at run time with an error message). | `cargo xtask wasm` passes in CI. | 4 |
| A4 | Groundwork ✅ 2026-10-13 (in W1: `App::with_engine`, `FileDialogs` with `ScriptedDialogs`, 8 kittest tests in `crates/ui-egui/tests/ui_smoke.rs`). **UI smoke tests** with `egui_kittest` 0.33 (matches egui 0.33). `App::new` takes an eframe `CreationContext` and the file dialogs block (rfd), so first split app construction from eframe and route dialogs through a replaceable provider. | Headless tests click every tool, draw a box, open each dialog and the unsaved-changes prompt; they run in `cargo test`. | 10 |
| A5 | **Export polish.** (a) Export bounds: miter joins and arrow tips can reach past the scene's bounding box (it adds only half the stroke width), and the default margin is 0, so tips touching the edge get cut; make bounds join-aware and add a small default border. (b) True curves: the scene holds only polylines (an ellipse of radius 600 becomes about 750 points), so add curve segments to `Prim::Path` and emit them in SVG, PDF and EPS (the raster backend keeps flattening). | Sample exports unclipped in all formats; the sample's SVG at least 50 % smaller; raster output unchanged. | 12 |
| A6 | **Crash safety:** autosave a recovery file after edits (debounced) and from the panic guard; offer recovery on start (xfig writes `SAVE.fig`). Native only; the web version gets browser autosave in C3. | Kill the app mid-edit; the next start offers to restore; recovery files are removed after a clean save or exit. | 6 |
| A7 | **No UI stalls on large drawings.** Today the canvas rebuilds the scene and re-rasterises the whole canvas on the UI thread whenever the view changes, so every pan or zoom step of the 50k-path document stalls for about 360 ms (30 ms scene, 330 ms raster; at fit zoom everything is visible, so culling alone won't help). Keep the scene per revision; while panning, shift the existing texture and draw only the uncovered strips; rasterise in a background thread (native) and swap the result in when ready; use a spatial index for culling when zoomed in. | UI frame time during pan and zoom < 16 ms on the 50k-path document; the full-quality image follows within 0.5 s. | 12 |
| A9 | **Per-command cost.** Commands call `fix_all_text`, which re-measures every text in the document after any edit; re-measure only texts that changed. Also make the hit-test bounding-box cache safe against changes inside a running command. | A one-object move on the measured document < 2 ms (now 5.9 ms, before A1). | 4 |
| A8 | **fig2dev reference tests.** Render test figures through fig2dev and RustFig and compare them (design below). | `cargo test` runs the comparison when fig2dev is installed and skips otherwise; each fixture has a recorded score; the diff images are written for review. | 12 |

## Phase W: xfig window parity (≈92 h)

RustFig's window has the same building blocks as xfig's (mode panel left,
attribute panel bottom, canvas, depth panel) but not xfig's layout or
behaviour. The xfig manual's screen map lists: main menus (File, Edit,
View, Snap, Help), current file name, message panel, mouse-function
indicator, units box, top and side rulers, drawing and editing mode panels,
attribute panel, depth panel, canvas. This phase makes RustFig's main window
match it, with the modern additions kept but optional.

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| W1 | ✅ 2026-10-13. **xfig layout** (starts with the A4 groundwork, see Order): a top strip with the current file name, the message panel (with history), the mouse-function indicator and the units box; menus File, Edit, View, Snap, Help in xfig's order and naming (RustFig's Object/Tools menus remain as extras); depth panel shown by default. A "Classic xfig / Modern" layout setting. | Side-by-side screenshot with the manual's screen image shows the same regions in the same places. | 16 |
| W2 | ✅ 2026-10-14 (`MouseScheme`, `Button`, `tool.mouse_buttons`; `mouse_hints_match_behaviour` checks every tool, state and scheme; arc by centre, freehand, delete region, cut buffer, copy & flip/rotate, update settings, break and tag added on the way). **Mouse setup and indicator**: xfig's three-button semantics per mode (e.g. polyline: left adds a point, middle finishes, right cancels; move: middle places at the original position) with the indicator showing what each button does in the current mode and state. Setting "Mouse buttons: xfig (3 buttons) / modern (left + double click)", plus modifier equivalents for two-button mice and trackpads. | For every tool the indicator text matches the tool's actual behaviour (unit test over all tools and states); both mouse schemes have tool tests. | 16 |
| W3 | ✅ 2026-10-14 (`depth.hide_all`, `depth.toggle_all`, `depth.set_display`, `depth.set_visible` with lists; export `active_layers_only` and `whole_figure_bounds`; printing comes with B2). **Depth panel (layers) parity**: a row for every used depth with object counts; click or drag across checkboxes to switch several depths; All On / All Off / Toggle buttons; Gray or Blank display of depths that are off; objects on off depths can't be picked or edited; export and print of "active layers only". | Tests for picking and exports with depths off; screenshot review. | 12 |
| W4 | ✅ 2026-10-14 (`engine/snap.rs`, ported from `w_snap.c`/`w_intersect.c`; normals, tangents and intersections of curves on the flattened outline; splines snap too; `snap.mode`, `snap.hold`, `snap.release`; Snap Mode box). **Snap menu**: Hold, Release, Endpoint, Midpoint, Nearest, Focus, Diametric, Normal, Tangent, Intersect, applied to the next clicked point in drawing and editing modes (replaces E10). | Tests per snap mode on lines, arcs, ellipses and splines. | 16 |
| W5 | ✅ 2026-10-14 (`ui-egui/attributes`, ported from `w_indpanel.c`; Arc Type and Text Step added; the attributes of later features — Angle Geom D7, Smart Links D8, copy counts and dimension line D10, tangent length D4 — appear with them). **Attribute panel parity**: xfig-style buttons that show the current value graphically and open a choice panel; only the attributes relevant to the current mode are shown, like xfig; adds the missing ones: point positioning (D7), linked move (D8), text flags (rigid, special, hidden), arrow mode and arrow-type panel, latex/manhattan line modes, depth. | Screenshot review per mode; switching modes changes the visible attributes. | 16 |
| W6 | ✅ 2026-10-14 (`ui-egui/rulers`, ported from `w_rulers.c` and `u_pan.c`; `view.set_scale`, `app.unit_panel`). **Rulers** (top and side) with cursor tracking, unit ticks and click-to-pan (replaces E2); **units box**: click to change units and scale. | Screenshot review; units change updates rulers and coordinates. | 10 |
| W7 | ✅ 2026-10-14 (`menus::XFIG_KEYS` from `app-defaults/Fig`; mode letters corrected to xfig's; `app.toggle_xfig_keys`, on by default). **Menu accelerators** matching xfig's (Meta-key shortcuts in the menus, e.g. Meta-N, Meta-U undo, Meta-T paste) as an optional keymap next to the platform-standard one. | Tests that the xfig keymap reaches the documented commands. | 6 |

Global settings (Edit › Global settings, E1), the character map (View, E5)
and Manage styles (View, E6) complete the xfig menus. E8 (mouse hint bar) is
covered by W2.

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
| D10 | **Remaining mode options** found by the source map: dimension lines (Shift+left in polyline mode, `d_line.c`), align to canvas (`e_align.c`), array copy (`e_copy.c`), anchor point for flip/rotate and number of rotated copies (`e_flip.c`, `e_rotate.c`), scaling one edge (`e_scale.c`), lengths shown while drawing (`w_msgpanel.c`), typed coordinates for the next point (`w_keyboard.c`). | A test per option; mouse hints list them. | 16 |
| D9 | Arrowhead types 10–14 with exact shapes; dash-dot tick styles. | Visual review against the xfig manual. | 6 |

## Phase E: UI completeness, M7 (≈56 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| E1 | **Settings persistence** (eframe storage on native and web): indicator, grid, units, panel visibility, recent files. | Restart keeps settings; File menu lists recent files. | 8 |
| E3 | **Zoom box** and zoom tool. | Drag a box to zoom into it. | 4 |
| E4 | **Colour editor**: create and edit user colours (RGB/HSV), remove unused ones. | New colours save to the Fig file. | 10 |
| E5 | **Character map** inserting into the text being typed (Symbol and Dingbats included). | Insert ∑ with the Symbol font. | 8 |
| E6 | **Style manager**: named attribute sets applied to the indicator. | Save, apply, delete a style; persists. | 10 |
| E7 | **System clipboard**: copy objects as Fig text and SVG, paste Fig text and images. | Copy between two RustFig windows; paste an image from a browser. | 10 |
| E8 | **Shortcut sheet** listing every command and its keys (the mouse indicator is W2). | Screenshot review. | 2 |
| E9 | **Dark/light theme switch** and UI scale setting. | Screenshot review. | 4 |

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

fig2dev (3.2.9a at the time of writing; Homebrew, apt and most TeX
distributions ship it) is
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

1. Phase W, xfig window parity, first: W1 → W2 → W3 → S1 → S2 → W5 → W6 → W4 → W7, with D7 and D8 alongside W5. W1 starts with the A4 groundwork (app construction separated from eframe, replaceable file dialogs, a first `egui_kittest` test) so every W task lands with UI tests.
2. A8 and B0 (fig2dev reference tests, then the fig2dev export backend).
3. A1 and A9 (undo memory, per-command cost), A2 and A3 (CI, web build), the rest of A4.
4. A5–A7, then B1, B2, C1–C3: the first release candidate for LaTeX users, on desktop and web.
5. D1, D6, E1, E7: the features xfig users miss most day to day.
6. The rest of B, D and E, then F and G toward Beta (parity ≥ 85 %, about 550 h in total).

## Decisions that are the user's

- Use of xfig's sources (decided 2026-10-14): agents may read, port and reuse xfig and fig2dev code under the xfig licence, keeping its notice (`LICENSE-xfig`). The figure libraries in `old/xfig-3.2.9a/Libraries/` have mixed or missing licence notes; bundling them is still the user's decision (with D6).
- New heavy dependencies (PDF rendering, spell checking, localization framework).
- CI cost (decided 2026-10-13): Linux runs on every push; Windows and macOS run only for releases (A2).
- fig2dev stays an optional external program (agreed 2026-10-13): never bundled, never required for the native formats or the web build.
- Publishing (demo site, crate names, releases) and the product name/branding.
- Licence changes and any asset of unclear provenance.

## Revision history

| Date | Note |
|------|------|
| 2026-10-13 | First plan, after M0–M3. |
| 2026-10-13 | Added A8 (fig2dev reference tests) and B0 (fig2dev export backend) with design notes. |
| 2026-10-14 | A3: the web build compiles locally with rustup's toolchain; xtask uses the running toolchain's rustc. |
| 2026-10-14 | W7 landed: xfig keymap; phase W done. |
| 2026-10-14 | W4 landed: xfig's Snap menu and Snap Mode box. |
| 2026-10-14 | W6 landed: xfig's rulers, units box and Unit menu. |
| 2026-10-14 | W5 landed: xfig's attribute panel. |
| 2026-10-14 | S2 landed: xfig's mouse indicator and wording. |
| 2026-10-14 | S1 landed: xfig's draw, save and pick order. |
| 2026-10-14 | Added phase S (source map S0 done; S1–S3) and D10; tasks port from xfig's sources where they can. |
| 2026-10-14 | xfig and fig2dev sources may now be read and ported (user decision; xfig licence, `LICENSE-xfig`). |
| 2026-10-14 | W3 landed: depth panel with counts, drag-toggle, All On/Off/Toggle, Gray/Blank; hidden depths can't be picked (Add Point could before); export of active layers only. |
| 2026-10-14 | W2 landed: xfig three-button semantics per mode, modern scheme as a setting (View › xfig mouse buttons), Alt/Ctrl+click for middle/right, indicator with one column per button. |
| 2026-10-13 | W1 landed: classic xfig layout, Snap menu (positioning and grid), mouse indicator, message history, units box, UI tests. |
| 2026-10-13 | Phase W moved to the front (user request); W1 absorbs the A4 groundwork. |
| 2026-10-13 | A2: Windows and macOS CI only for releases (user decision). |
| 2026-10-13 | Reviewed phase A against the code: measured A1, corrected A5 and A7 (the root causes were different), added A9; phase A now ≈84 h. |
| 2026-10-13 | Added phase W (xfig window parity: layout, mouse setup and indicator, depth panel, Snap menu, attribute panel, rulers, accelerators); moved E2 and E10 into it. |
