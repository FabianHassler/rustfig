# Execution plan

**Status:** 2026-10-15. Phase W (with W8, W9), phase A (A1–A10) done, A8, A9, A10, D9 and D12 done; B0 (porting fig2dev) under way (options ported), then the rest of phase A. Overall parity ≈60 %.

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
| S3 | **xfig's command-line options, backwards compatible** (`main.c`, `xfig.man.in`; widened 2026-10-15 at the user's request: RustFig must be usable as a drop-in for `xfig`): RustFig accepts xfig's standard options (`-metric`/`-inches`, `-specialtext`, `-latexfonts`, `-startfontsize`, `-startgridmode`, `-startposnmode`, `-zoom`, `-exportLanguage`, arrow defaults, `-portrait`/`-landscape`, …) as a general compatibility feature; they override the preferences (E1) for that run. Nothing user-specific is built in. Backwards compatible beyond the options: every option of `xfig.man.in` accepted (X toolkit ones such as `-geometry`, `-display`, `-iconic` accepted and mapped or ignored with a note), xfig's resources from `app-defaults/Fig` and `~/.Xdefaults`-style `Fig.*` settings read where they have an equivalent, `~/.xfigrc` imported into the preferences on first start, an `xfig` command name (a launcher or symlink installed alongside `rustfig`), files saved exactly as xfig 3.2 saves them, and old Fig formats read (Fig 1.x/2.x, gap #13, G1). | Each option has a test; `rustfig -metric file.fig` behaves as `xfig -metric file.fig`; every option in `xfig.man.in` is accepted; a table in `docs/ui-parity.md` lists each option with its effect. | 16 |
| S4 | **Function-level source audit** (asked 2026-10-15). S0 maps files; this goes function by function through xfig's `src/*.c` and fig2dev's `fig2dev/*.c` and `dev/*.c` (about 3,300 function definitions: 2,300 in xfig, 1,000 in fig2dev) and records for each: **ported** (RustFig module and item), **replaced** by a deliberate modern equivalent (with the reason, e.g. X toolkit widget code → egui, `u_print.c` printing → PDF to the system print dialog (B5), ghostscript calls → RustFig's renderer, `~/.xfigrc` → TOML preferences (E1)), **not needed** (X11/Xt plumbing, memory management, platform workarounds), or **missing** (with the task that ports it; new tasks where none exists). Generated with a script that lists every C function (ctags-style) into `docs/source-audit.md`, then reviewed by hand; the S0 file statuses are corrected from it. | Every C function has a status; no "missing" function without a task; `source-map.md` statuses agree; the script can be rerun to catch functions added upstream. | 32 |

## Phase A: hardening (≈84 h)

Checked against the code on 2026-10-13. Measured on a document of 50 000
polylines and 2 000 texts (release build): one undo snapshot is about
14.5 MB and takes 9 ms to copy; a one-object `transform.move` costs 5.9 ms,
most of it spent copying the document and re-measuring every text.

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| A1 | ✅ 2026-10-15. **Shared undo snapshots.** Top-level objects are `Arc<Object>`; commands copy only what they change (`Arc::make_mut`, `Document::get_mut`; `fix_all_text` copies only objects whose text metrics change). `history.rs` stores each step as a splice (shared prefix and suffix, the differing run kept) and caps the history at 200 steps and about 256 MB (`Engine::undo_bytes`). Measured: 200 one-object moves on 50,000 objects hold 0.08 MB of undo (`tests/edit_cost.rs`); `rustfig-cli perf` edit 0.4 ms, undo 0.4 ms. Was: `Engine::execute` deep-copies the whole document before every mutating command, and `History` keeps up to 200 copies: about 2.9 GB for the measured document. Store top-level objects as `Arc<Object>` so a snapshot copies pointers, and mutate through `Arc::make_mut` only on the objects a command touches (about 15 `doc_mut().objects.iter_mut()` call sites in `cmd/*`). Cap the history by memory as well as by step count. | 200 single-object edits on the 50k-path document add < 50 MB; snapshot cost < 1 ms; `rustfig-cli perf` gains an "edit + undo" row within budget. | 10 |
| A2 | ✅ 2026-10-15: `.github/workflows/ci.yml` (every push and pull request: Linux, system packages for eframe and rfd's GTK dialogs, fig2dev, wasm target, `RUSTFIG_REQUIRE_WASM=1`, `cargo xtask ci`, perf budgets, rust-cache; fig2dev comparison images uploaded on failure) and `release.yml` (`v*` tags and by hand: the gates on Linux, Windows and macOS, release builds packaged with the licences, attached to the GitHub release). Verified: CI green on Linux (run 38653453406, after installing the PostScript fonts' URW clones for the A8 text fixture); the release workflow, run by hand (38658367369), passed the gates and packaged binaries on Linux, Windows and macOS (6–10 MB each). **CI pipeline** (GitHub Actions; none existed). **Every push and pull request:** `cargo xtask ci` on Linux, with the wasm target installed, `RUSTFIG_REQUIRE_WASM=1`, fig2dev installed for A8, and a cargo cache. **Releases only** (tags `v*`, plus manual `workflow_dispatch`): the same on Windows and macOS, followed by release builds attached to the GitHub release. Linux needs system packages for eframe/glow and for `rfd` (GTK 3 by default; switching `rfd` to its xdg-portal backend avoids GTK). | Linux job green on every push with the wasm check running, not skipped; pushing a `v*` tag runs all three OSes and produces release binaries. | 8 |
| A3 | ✅ 2026-10-15: `cargo xtask wasm` runs in CI with `RUSTFIG_REQUIRE_WASM=1` and passes. Checked: the web entry point (`eframe::WebRunner` on `#rustfig_canvas`), eframe's `wayland`/`x11` features (native only), time and threads gated, `std::fs` returning errors (not panics) on wasm. Earlier: 2026-10-14: compiles locally (`cargo xtask ci` runs the wasm check with rustup's stable toolchain); the CI step remains. **Web build compiles.** Code review found no blockers so far: native-only code (rfd, control channel, `Instant`) is cfg-gated and `getrandom` isn't in the wasm dependency tree. Still to confirm: the eframe `wayland`/`x11` features on wasm, `eframe::wasm_bindgen` in `apps/rustfig`, and `std::fs` calls (they compile, then fail at run time with an error message). | `cargo xtask wasm` passes in CI. | 4 |
| A4 | ✅ 2026-10-15: every dialog opens headless (Export, Search and Replace, Document Settings, Commands, About, Global settings, Unit menu, Edit attributes), Save As/Open/Merge/Import picture through `ScriptedDialogs` (including a cancelled one), Quit with and without unsaved changes (`App::quit_requested`); 27 kittest tests. Groundwork ✅ 2026-10-13 (in W1: `App::with_engine`, `FileDialogs` with `ScriptedDialogs`, 8 kittest tests in `crates/ui-egui/tests/ui_smoke.rs`). **UI smoke tests** with `egui_kittest` 0.33 (matches egui 0.33). `App::new` takes an eframe `CreationContext` and the file dialogs block (rfd), so first split app construction from eframe and route dialogs through a replaceable provider. | Headless tests click every tool, draw a box, open each dialog and the unsaved-changes prompt; they run in `cargo test`. | 10 |
| A5 | ✅ 2026-10-15. (a) The export bounds are fig2dev's since B0 (`render/bound.rs`, arrows and joins included, bounding boxes equal fig2dev's), so nothing is clipped; fig2dev adds no default border, nor does RustFig. (b) `SubPath::curve` holds the exact outline as lines and cubic Béziers (`geom::curves`: ellipses, arcs, pie wedges, rounded boxes; arcs shortened by arrowheads keep their points) and SVG, PDF and EPS write it; the raster backend keeps the flattening. Ellipses fixture SVG 169 KB → 2 KB, arcs 59 → 22 KB, boxes −28 %; corpus EPS median 0.0031 → 0.0028, 95th percentile 0.031 → 0.029, PNG unchanged. **Export polish.** (a) Export bounds: miter joins and arrow tips can reach past the scene's bounding box (it adds only half the stroke width), and the default margin is 0, so tips touching the edge get cut; make bounds join-aware and add a small default border. (b) True curves: the scene holds only polylines (an ellipse of radius 600 becomes about 750 points), so add curve segments to `Prim::Path` and emit them in SVG, PDF and EPS (the raster backend keeps flattening). | Sample exports unclipped in all formats; the sample's SVG at least 50 % smaller; raster output unchanged. | 12 |
| A6 | ✅ 2026-10-15. `engine::recovery` (native): each running instance holds an exclusive OS lock on `ID.lock` in the recovery folder (macOS `~/Library/Application Support/RustFig/recovery`, Linux `$XDG_DATA_HOME/rustfig/recovery`, Windows `%LOCALAPPDATA%\\RustFig\\recovery`) and keeps `ID.rustfig` (lossless) and `ID.path` there; the system frees the lock however the process ends, so the next start offers the drawing at once. `ui-egui::autosave` writes the copy 3 s after the last change and from the panic guard, removes it after saving and on a clean exit; the start dialog "Recover unsaved drawings" restores (unsaved, under the original name) or discards; commands `file.recoverable`, `file.recover`, `file.discard_recovery` (ids validated, never paths). Verified: drew a box, `kill -9`, restarted: offered and restored. **Crash safety:** autosave a recovery file after edits (debounced) and from the panic guard; offer recovery on start (xfig writes `SAVE.fig`). Native only; the web version gets browser autosave in C3. | Kill the app mid-edit; the next start offers to restore; recovery files are removed after a clean save or exit. | 6 |
| A7 | ✅ 2026-10-15. The canvas keeps the scene per drawing (not rebuilt on pan or zoom), shows its last raster moved and scaled to the current view at once, and renders the exact one on a background thread (native, the newest view wins); drawings that render in under 20 ms stay synchronous. Measured on a 50,000-object file over the control channel (`ui.state` now reports `frame_ms` and `frame_ms_max_recent`): frames while panning 1–2.6 ms, zoom steps about 2 ms (before: each step blocked for the 330 ms render); the exact image follows in about 0.33 s. Spatial-index culling wasn't needed. **No UI stalls on large drawings.** Today the canvas rebuilds the scene and re-rasterises the whole canvas on the UI thread whenever the view changes, so every pan or zoom step of the 50k-path document stalls for about 360 ms (30 ms scene, 330 ms raster; at fit zoom everything is visible, so culling alone won't help). Keep the scene per revision; while panning, shift the existing texture and draw only the uncovered strips; rasterise in a background thread (native) and swap the result in when ready; use a spatial index for culling when zoomed in. | UI frame time during pan and zoom < 16 ms on the 50k-path document; the full-quality image follows within 0.5 s. | 12 |
| A9 | ✅ 2026-10-15. **Per-command cost.** `Engine::execute` re-measures texts only in top-level objects the command changed (not pointer-shared with the snapshot; commands no longer call `fix_all_text`); the hit-test box cache keeps each box with its object (`Arc`), recomputing only changed ones, so it is right inside a running command too (`tests/edit_cost.rs`). One-object move with 5,000 texts: 1.7 → 0.4 ms (`rustfig-cli perf`). Was: Commands call `fix_all_text`, which re-measures every text in the document after any edit; re-measure only texts that changed. Also make the hit-test bounding-box cache safe against changes inside a running command. | A one-object move on the measured document < 2 ms (now 5.9 ms, before A1). | 4 |
| A10 | ✅ 2026-10-14. **EPS and PDF compared with fig2dev on xfig's sample figures** (asked 2026-10-14): `crates/engine/tests/fig2dev_corpus.rs` (`#[ignore]`d; `cargo test --release -p rustfig-engine --test fig2dev_corpus -- --ignored --nocapture`, about 12 minutes). Corpus: the A8 fixtures, fig2dev's `tests/data`, xfig's `Libraries` (from `old/` until D6 copies them; then from `libraries/`, so CI has them); `RUSTFIG_CORPUS_FILTER`, `RUSTFIG_CORPUS_LIMIT` narrow it. Per figure: fig2dev's EPS (run with xfig's `-F`) and RustFig's EPS, both rasterised by ghostscript at 150 dpi; RustFig's PNG against fig2dev's EPS; both PDFs through ghostscript; the bounding boxes. Report `target/fig2dev-reference/corpus/index.html`, worst first, images of the worst 150; summary (median, 95th percentile per measure, figures with errors) in `fixtures/corpus-scores.json`, checked when the corpus size matches. Ghostscript and fig2dev are test-only and optional. Findings: metric text was magnified (fixed: fig2dev keeps its point size); files without the centring header line were refused (fixed, as fig2dev reads them); fill patterns differ most (D12); RustFig's tight bounding box crops lines at the edge (B0 `bound.c`); text widths (B8). Baseline over 2,569 figures: EPS median 0.015, 95th percentile 0.072. 2026-10-15: fig2dev now runs with `-E 1` (ISO-8859-1, what the Fig 3.2 format prescribes without an `#encoding: UTF-8` line): without it fig2dev guessed UTF-8 from the locale and dropped Latin-1 characters (the Ø in Mechanical_DIN's 59 figures, whose EPS median difference fell from 0.036 to 0.009). | Runs over the whole corpus; the report lists every figure; B0–B9 use it to check each ported driver. | 10 |
| A8 | ✅ 2026-10-14. **fig2dev reference tests** (`crates/engine/tests/fig2dev_reference.rs`): 11 fixtures rendered by fig2dev (its SVG, rasterised with resvg: no ghostscript) and by RustFig, trimmed, scaled, blurred and compared; baselines in `fixtures/scores.json`, `UPDATE_REFERENCE=1` rewrites them; images in `target/fig2dev-reference/`. Skips without fig2dev. First findings: line widths (fixed: fig2dev's `linewidth_adj`), arrowhead types ≥ 4 and arrows on pie-wedge arcs (D9), Symbol font (B8), arc direction flag (D9). | `cargo test` runs the comparison when fig2dev is installed and skips otherwise; each fixture has a recorded score; the diff images are written for review. | 12 |

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
| W8 | ✅ 2026-10-15. `docs/keys-parity.md` lists every binding of `app-defaults/Fig` and the 210 of the C translation tables with its status; every one now works, is replaced by a modern equivalent (system file dialogs, Cmd/Ctrl+V, platform text-field keys), or waits for the feature it needs, with that task named (B5 print, D6 libraries, D10 lengths, D11 digitize/spell check/keyboard panel, E3 zoom box, E4 colour editor, E6 styles, E1 recent files). Added: canvas arrows and Home; Home/End while typing; Meta-F/E/V/H and menu letters; Shift-Meta-X (`app.export_now`); Meta-Y (`app.toggle_balloons`); Alt+wheel mode menus; the compose key (`rustfig_text::compose`, 97 sequences from `CompKeyDB`); wheel on indicator buttons and spinners; Return/Escape in the edit, export, search, unit and message panels; depth panel right click. **xfig's translations and accelerators, checked one by one** (asked 2026-10-15). W7 ported the menu accelerators; this audits every key and mouse binding xfig defines and closes the gaps. Sources: `app-defaults/Fig` (`Fig*accelerators`, about 50; the letter keys while a menu is open, `Fig*commands*{File,Edit,View,Help}*translations`, including File's recent files 1–9 and View's `v,v`; `*Text*translations` Delete/End/Home; the found-text and spell-check panels; the wheel bindings: indicator buttons, spinners, Alt/Meta+wheel popping up the draw/edit mode panels, Ctrl+K keyboard panel), the ~114 `XtParseTranslationTable`/`XtParseAcceleratorTable` tables in `src/*.c` (Return/Escape and field keys in every panel, `e_edit.c` popups, file, export, print, colour, font, style, search panels, rulers, depth panel), the canvas key handling (`w_canvas.c`, `w_keyboard.c`) and `CompKeyDB` (compose sequences for text). Deliverable: `docs/keys-parity.md` listing every binding with its xfig source, the RustFig command or handler, status and task; missing ones implemented (or given a task where they need a missing feature, e.g. spell check in D11); a table-driven test that every app-defaults accelerator and menu letter reaches its command. | Every binding in `app-defaults/Fig` and the C translation tables is listed with a status; all whose feature exists work and are tested; `ui-parity.md` links the list. | 10 |
| W9 | ✅ 2026-10-15. All 107 of xfig's bitmaps have vector versions, drawn by RustFig's rasteriser at the screen's density and cached per scale: the attribute pictures from real objects (`vector_icons.rs`: line styles, joins, caps, arrows of all 15 types, arc types, justification), the mode icons and the grid/position/alignment pictures traced from the bitmaps (`mode_icons.rs`; every one covers its bitmap at 95–100 % within a pixel, `traced_icons_match_xfigs_bitmaps`; lettering and pixel drawings such as the camera, pencil, blade and books are kept as crisp squares). Global settings › Sharp attribute pictures returns to xfig's bitmaps. Arrow types 10–14 became selectable. The application icon is an original drawing generated in code (`app_icon.rs`), set as the window icon; `.icns`/`.ico` files for packaging remain (C/A2 release packaging). **Sharper icons** (asked 2026-10-15). The mode and attribute icons are xfig's 1-bit 36×36 bitmaps (`xfig_icons.rs`), drawn one bitmap pixel per point with nearest-neighbour sampling: crisp at 1×, jagged on HiDPI screens and at larger UI scales. Redraw them as vectors in code, keeping xfig's designs: (a) the attribute pictures (line styles and widths, joins, caps, arrow types, fill shades and patterns, arc types, justification, grid, depth) are drawn by RustFig's own renderer from real objects (`geom::arrowhead`, `dash_pattern`, `pattern::tile`), so they match the canvas exactly; (b) the mode icons (about 40) become small vector drawings (lines, arcs, X-splines, text) traced from the bitmaps; (c) icons are rasterised with tiny-skia at the screen's pixel density and cached per scale. Original work under MIT OR Apache-2.0, generated in code (no asset files; `ASSETS.md` and `NOTICE` updated); xfig's bitmaps stay available ("xfig bitmaps" in Global settings, saved with E1) for a pixel-exact classic look. | Each vector icon, rasterised at 1×, overlaps its xfig bitmap by at least 90 % (a test over all icons); screenshots at 1×, 2× and 3× reviewed; no texture rebuilt per frame. | 16 |

Global settings (Edit › Global settings, E1), the character map (View, E5)
and Manage styles (View, E6) complete the xfig menus. E8 (mouse hint bar) is
covered by W2.

## Phase B: exports (fig2dev ported), M4 (≈160 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| B0 | 🟡 2026-10-14: language table, LaTeX box, pspdf, `file.export_formats`, export dialog, CLI `formats`; fig2dev's bounds (`bound.c`, `render/bound.rs`) size every export: 2,532 of 2,568 corpus bounding boxes now equal fig2dev's, the rest within 1 pt. 2026-10-15: fig2dev's options (`render/options.rs`, from `fig2dev.c`, `genps.c`, `genpdf.c`, `genbitmaps.c`, `gensvg.c`): `file.export` takes `options` by key or letter, `file.export_formats` lists each language's options with status, CLI `--option KEY=VALUE` and `formats LANG`; ported: `-D`, `-K`, `-m`, `-Z`, `-s`, `-b`, `-g` (X11 colour names from `colors.c`), `-N`, `-n`, `-f`, `-F`; the rest are refused with their task (page layout B4, previews B4, bitmaps B6). Bounding boxes with `-b`, `-m`, `-Z`, `-D`/`-K` equal fig2dev's. Next: `texfonts.c` with B1–B3. **Port fig2dev to Rust** (decided 2026-10-14; design below): no external fig2dev and no ghostscript at runtime, so every format works on every platform and on the web. Start with the shared parts: `fig2dev.c`'s per-language options, `bound.c` (bounds with arrows and text), colour and font tables (`colors.c`, `psfonts.c`, `texfonts.c`), the export-format list and file-name mapping, `file.export_formats`. B1–B4, B6 and B9 then port the drivers. | `file.export_formats` lists every fig2dev language with its status; the export dialog greys out the ones not yet ported. | 10 |
| B1 | **TikZ export** (`.tikz`/`.tex`): paths, fills, patterns, arrows, dashes, text with special flag passed through raw. | Compiles with pdflatex; a visual diff against the PDF export is close. | 16 |
| B2 | **PDF/EPS + LaTeX overlay** (fig2dev's pdftex_t/pstex_t): graphics without special text, plus a `.tex` file placing the text. | The combined output typesets in LaTeX with fonts from the document. | 12 |
| B3 | **PSTricks and pict2e exports.** | Both compile; a test checks the structure. | 14 |
| B4 | **PostScript with page layout:** orientation, centring, paper size, magnification, multi-page tiling (the Fig header's "Multiple"). | Output matches the header settings; ghostscript renders it (manual check). | 12 |
| B5 | **Printing, modernised** (decided 2026-10-14): xfig's Print panel options (paper, orientation, magnification, centring, layers, copies) produce a PDF that goes to the operating system's print dialog on macOS, Windows and Linux (and the browser's print on the web), instead of xfig's PostScript spooler commands. Command `file.print`. | Prints on all three desktop platforms; the web build opens the browser's print dialog. | 12 |
| B6 | **Bitmaps without ghostscript** (`genbitmaps.c` ran ghostscript): GIF (with the Fig transparent colour), JPEG (quality), TIFF, PPM, XBM, XPM, PCX from RustFig's own renderer. | Round trip through the `image` crate in tests; the reference test compares them with fig2dev. | 8 |
| B9 | **Port the remaining fig2dev drivers**: the LaTeX picture languages (`genlatex.c`, `genepic.c` with eepic/eepicemu, `genpictex.c`, `gentextyl.c`, `gentpic.c`, `genpic.c`), EMF, CGM, DXF, HPGL (`genibmgl.c`), MetaPost, METAFONT, image maps, Tk/Perl-Tk, shape, Gerber, GE, box. | Each language has a structural test and, where it renders, a reference comparison with fig2dev. | 60 |
| B10 | **A standalone `fig2dev`** (asked 2026-10-15): a `fig2dev` binary (`apps/fig2dev`) built on the ported drivers that accepts fig2dev 3.2.9a's whole command line as the original does, so scripts, Makefiles and LaTeX workflows (`fig2ps2tex`, `pic2tpic` style) work unchanged: `-L lang` and the language guessed from the output suffix, every general and per-language option with fig2dev's letters and argument forms (`-m`, `-Z 4w`, `-D +10,40:50`, `-G .25:1cm`, `-b`, `-g`, …), input and output files or stdin/stdout (`-`), `-h`/`-V`/`--version` output, its exit codes and its error messages. Options are parsed by one shared table (`render/options.rs`, already keyed by fig2dev's letters), so `rustfig-cli convert --option` and `fig2dev` stay in step. Options of drivers not ported yet fail with fig2dev's message plus the plan task. | fig2dev's own test suite (`fig2dev/tests/*.at`) run against RustFig's `fig2dev`, with the outputs compared where the drivers are ported (A10's comparison); every option in fig2dev's man page parsed (a test per option); `fig2dev -L eps in.fig out.eps` gives the same bounding box as the original. | 12 |
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
| D6 | **Library objects** (decided 2026-10-14: bundle xfig's libraries). Copy xfig's `Libraries/` (33 libraries, ≈2,600 files, 15 MB) into `libraries/` with each library's README/LICENSE notes; the few pictures they use (GIF, PNG, XPM, XBM, PS) get `ASSETS.md` rows. Library mode as xfig's (`e_placelib.c`, `w_library.c`): browse with thumbnails, place copies (place at original position with Shift+right), copy a selection to a library; user library directories from the preferences. | Place an object from a bundled library; undo removes it; `cargo xtask assets` passes. | 30 |
| D7 | **Point positioning modes**: latex lines, manhattan, mountain, 45° constraint with Shift. | Tests per mode. | 10 |
| D8 | **Linked move**: moving a box drags the attached line end points (xfig's smart links). | Test with a box and two attached lines. | 10 |
| D10 | **Remaining mode options** found by the source map: dimension lines (Shift+left in polyline mode, `d_line.c`), align to canvas (`e_align.c`), array copy (`e_copy.c`), anchor point for flip/rotate and number of rotated copies (`e_flip.c`, `e_rotate.c`), scaling one edge (`e_scale.c`), lengths shown while drawing (`w_msgpanel.c`), typed coordinates for the next point (`w_keyboard.c`). | A test per option; mouse hints list them. | 16 |
| D11 | **Every remaining xfig function** (goal decided 2026-10-14: all of xfig's functions): Digitize (`w_digitize.c`, tablet input against a scanned figure), screen capture into a picture object (`w_capture.c`, through the OS screenshot facilities), Show vertex numbers, Show info balloons, Autorefresh mode (`w_cmdpanel.c`), edit the figure comments from Edit mode, Unzoom semantics. Modern equivalents where the X11 way no longer works, decided per item. | Each menu entry and mode works; none is greyed out any more. | 24 |
| D12 | ✅ 2026-10-15. **Fill patterns as fig2dev's PostScript draws them** (found by A10): the 22 patterns ported from `psprolog.h` (`FILL_PAT01`…: cells, line widths 0.7–1 pt, arcs) into `render/pattern.rs`; in exports the tiles keep their physical size and start at the page's lower-left corner (`PatternFrame`), as in fig2dev. At 600 dpi the ink coverage equals fig2dev's (horizontal lines 0.288 vs 0.284, circles 0.145 vs 0.142). Corpus: Welding `bi` 0.16 → 0.000, Optics grating 0.156 → 0.0025, `Examples/patterns.fig` 0.152 → 0.047; fig2dev's `tests/data/patterns.fig` 0.296 → 0.087 (the rest is the comparison's trim-and-rescale drift on a dense figure). The A10 test now rasterises both PostScript files without graphics anti-aliasing (ghostscript draws pattern tiles without it anyway). | The pattern figures match fig2dev's; `Examples/patterns.fig` below 0.05. | 8 |
| D9 | ✅ 2026-10-14. **Arrowheads, arcs and dashes as fig2dev draws them** (found by A8): arrowheads ported from fig2dev `bound.c` (`arrow_shapes`, `calc_arrow`: all 15 types, tip moved back by the miter, half heads flush with the line, type 9's half fill) and `genps.c` (hollow heads filled white); arrows on pie-wedge arcs; polylines aim the head along their last segment; arcs follow their direction flag; dash patterns and dotted caps from `genps.c` `set_style`. Scores: arrows 0.0169 → 0.0077, arcs 0.0087 → 0.0041. | The `arrows` and `arcs` reference fixtures score below 0.01 (fig2dev's SVG rounds coordinates, so not below 0.005). | 8 |

## Phase E: UI completeness, M7 (≈56 h)

| Id | Task | Acceptance | Hours |
|----|------|------------|------:|
| E1 | ✅ 2026-10-15. `settings.toml` (macOS `~/Library/Application Support/RustFig/`, Linux `$XDG_CONFIG_HOME/rustfig/`, Windows `%APPDATA%\\RustFig\\`; the browser's storage on the web). `[xfig]` holds xfig's resources under xfig's names (`engine::prefs::RESOURCES`, 67 from `xfig.man.in` and `main.c`, each applied or kept for the task that uses it; 17 X11-only ones removed at the user's request and dropped quietly from files); `[rustfig]` holds layout, mouse buttons, keymap, panels, icon style, recent files (9, File menu, 1–9) and the user's colours (asked by the user: offered in every colour menu, added to the drawing as Fig user colours). Unknown keys are kept; a damaged file gives the defaults and a message. Edit › Global settings: "xfig resources…" (every resource with value, default, option, status), "Use current attributes as defaults", Your colours. Commands `prefs.get`, `prefs.set`, `prefs.use_current_attributes`, `prefs.add_color`, `prefs.remove_color`. **Preferences that survive restarts** (a modernisation: xfig keeps only recent files in `~/.xfigrc` and otherwise relies on X resources). One store behind an engine interface: a settings file in the platform's config directory (`~/.config/rustfig/`, `~/Library/Application Support/RustFig/`, `%APPDATA%\RustFig\`), browser storage on the web; xfig's standard defaults when it is missing; a damaged file is reported, never fatal. Stores what xfig makes configurable through resources (units, default font/size, LaTeX fonts, special text, grid, positioning, zoom, arrow defaults, export format), RustFig's switches (layout, mouse scheme, keymap, panels) and recent files. Edit › Global settings shows them; "Use current attributes as defaults" saves the attribute panel. | Restart keeps settings on macOS, Linux, Windows and the web; File menu lists recent files; a corrupt file falls back to defaults with a message. | 12 |
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

### B0: porting fig2dev (decided 2026-10-14)

- fig2dev's drivers are ported to Rust from `old/fig2dev-3.2.9a/fig2dev/` (xfig licence). RustFig never runs fig2dev or ghostscript: bitmap formats come from RustFig's renderer, PostScript and PDF from its own writers.
- **Module** layout: shared code in `rustfig-render` (bounds, colour and font tables, options), one module per driver (`render/src/drivers/<lang>.rs`) with a header comment naming its C source. Drivers write text or bytes; none needs native-only code, so all work on the web.
- **Options:** each language's fig2dev options (`fig2dev.c`, the man page) map to an `options` object on `file.export`; unknown keys are errors.
- **Commands:** `file.export` and `file.export_data` accept every language; `file.export_formats` lists them with extension and status; `file.export_latex_overlay` {path, kind: pdftex|pstex} writes the graphics and the `_t` file.
- **Formats from file names:** `.tikz`, `.pstricks`, `.cgm`, `.emf`, `.dxf`, `.mp`, `.mf`, `.gif`, `.jpg`/`.jpeg`, `.tif`/`.tiff`, `.ppm`, `.pcx`, `.xbm`, `.xpm`, `.ps`, `.pic`, `.tk`. A bare `.tex` is ambiguous, so it needs an explicit `format`.
- **CLI:** `rustfig-cli convert IN OUT [--format LANG] [--option key=value]…`, and `rustfig-cli formats`.
- **Tests:** per driver, a structural test, the command sweep with hostile `format`/`options`, and the A8 comparison against fig2dev where it is installed.
- **Docs:** `file-format-parity.md` rows per language; `source-map.md` status per `dev/*.c`.

### A8: reference tests (done 2026-10-14)

- **Fixtures** `crates/engine/tests/fixtures/*.fig`, written for RustFig: lines (widths, dashes, caps, joins), arrows (types 0–14, both styles), fills (shades, tints, patterns), splines (all six kinds), arcs (open and pie, both directions, arrows), ellipses (rotated), text (justification, angle, fonts), depths and compounds, colours (standard and user), metric, boxes (arc-box, polygon).
- **Rendering:** fig2dev `-L svg` (no ghostscript), rasterised with resvg (a dev-dependency) at 1000 px; RustFig's PNG export at 150 dpi on white. fig2dev is test-only and optional (`RUSTFIG_FIG2DEV`, `PATH`, the usual install directories).
- **Comparing:** trim to the ink, scale to a common size, grey, blur by 1 px; the mean absolute difference and the share of strongly differing pixels.
- **Baselines** `fixtures/scores.json`; the test fails when a score gets worse by more than 0.02. `UPDATE_REFERENCE=1` rewrites them after an intended change.
- **Review output:** `target/fig2dev-reference/<fixture>/{fig2dev,rustfig,diff}.png` (diff: red only in fig2dev, blue only in RustFig).
- **Expected differences:** fonts (RustFig's fallback faces versus fig2dev's PostScript names), pattern phase, and where fig2dev's SVG driver departs from its PostScript driver, which RustFig follows (dash lengths).
- **CI:** install fig2dev on the Linux runner (A2) so the comparison runs there; it skips elsewhere.

## Order and milestones

1. ✅ Phase W, xfig window parity (W1–W7, S0–S2), done 2026-10-14.
2. ✅ A8 (fig2dev reference tests), ✅ D9 (the arrowhead, arc and dash differences it found), ✅ A10 (EPS/PDF compared with fig2dev on xfig's sample figures), then B0 (the fig2dev port: language table ✅, `bound.c` ✅), ✅ D12 (fill patterns).
3. ✅ A1 and A9 (undo memory, per-command cost), ✅ A2 and A3 (CI, web build check), ✅ W8 (xfig's translations and accelerators), ✅ W9 (sharper icons), (CI on Linux, Windows and macOS; web build), the rest of A4, A5–A7.
4. ✅ E1 and S3 (preferences ✅, xfig's command line and backwards compatibility), S4 (function-level source audit).
5. B1–B10 (fig2dev's drivers ported, and a standalone `fig2dev` with the original's command line: LaTeX exports, bitmaps, PostScript; modern printing), C1–C3 (web): the first release candidate, on desktop and web.
6. D6 (libraries), D1–D5, D7–D12, E3–E9: every remaining xfig function.
7. F and G toward Beta and 1.0 (every xfig function available; about 550 h in total).

## Decisions that are the user's

- Use of xfig's sources (decided 2026-10-14): agents may read, port and reuse xfig and fig2dev code under the xfig licence, keeping its notice (`LICENSE-xfig`).
- Scope (decided 2026-10-14): RustFig makes **every xfig function** available, with xfig's behaviour and standard defaults; modernisations are deliberate decisions. It is a general, cross-platform tool: no user's personal options or platform special cases are built in; personal choices go into the preferences (E1, S3).
- Printing (decided 2026-10-14): modernised, PDF to the operating system's print dialog (B5).
- Libraries (decided 2026-10-14): xfig's figure libraries are copied into the repository and bundled (D6), keeping each library's notes.
- Preferences file format (decided 2026-10-14): TOML, a human-readable settings file that can be edited by hand like xfig's resources (E1).
- Outlook, not planned (2026-10-14): a WYSIWYG editor for LaTeX text in figures.
- New heavy dependencies (PDF rendering, spell checking, localization framework).
- CI cost (decided 2026-10-13): Linux runs on every push; Windows and macOS run only for releases (A2).
- PostScript is the reference (decided 2026-10-14): where fig2dev's drivers disagree (for example the SVG driver draws dashes at 2/3 of their length), RustFig follows `genps.c`, which matches xfig's screen. The A8 tests compare with fig2dev's SVG, so such differences are expected there.
- fig2dev is ported to Rust (decided 2026-10-14, replacing the 2026-10-13 plan to call it as an external program): no runtime dependence on fig2dev or ghostscript. The installed fig2dev serves only as the optional reference in the A8 tests.
- Publishing (demo site, crate names, releases) and the product name/branding.
- Licence changes and any asset of unclear provenance.

## Revision history

| Date | Note |
|------|------|
| 2026-10-13 | First plan, after M0–M3. |
| 2026-10-13 | Added A8 (fig2dev reference tests) and B0 (fig2dev export backend) with design notes. |
| 2026-10-14 | A3: the web build compiles locally with rustup's toolchain; xtask uses the running toolchain's rustc. |
| 2026-10-15 | D12 landed: fig2dev's fill patterns; corpus EPS median 0.0031, 95th percentile 0.031 (both PostScript files rasterised alike). |
| 2026-10-15 | A4 done: UI smoke tests cover every dialog, the file dialogs and quitting. |
| 2026-10-15 | Gap 23 closed: start values follow xfig's code. |
| 2026-10-15 | E1 done: preferences as xfig resources in TOML; user colours; recent files. |
| 2026-10-15 | A7 done: no UI stalls; phase A complete. |
| 2026-10-15 | A6 done: crash recovery. |
| 2026-10-15 | A10: fig2dev run with the Fig format's ISO-8859-1 input encoding (Mechanical_DIN no longer the worst library). |
| 2026-10-15 | A5 done: true curves in SVG, PDF and EPS. |
| 2026-10-15 | W9 done: every xfig bitmap vectorised; application icon. |
| 2026-10-15 | W9 widened (user): vectorize every remaining bitmap, add an application icon. |
| 2026-10-15 | W9 started: attribute pictures as vectors; arrow types 10–14 selectable. |
| 2026-10-15 | W8 done: every xfig binding audited and either working, replaced, or waiting for a named task. |
| 2026-10-15 | A2 and A3 done: CI green on Linux; release workflow verified on all three OSes; wasm check in CI. |
| 2026-10-15 | W8 started: binding audit, canvas keys, quick export, wheel on indicator, Escape and Return in panels. |
| 2026-10-15 | Added S4 (function-level source audit: ported, replaced by a modern feature, not needed, or missing) and B10 (standalone `fig2dev` with the original's command line); S3 widened to full xfig backwards compatibility. All asked by the user. |
| 2026-10-15 | Added W9: vector icons at the screen's pixel density, asked by the user. |
| 2026-10-15 | Added W8: audit of xfig's translations and accelerators (app-defaults and the C translation tables), asked by the user. |
| 2026-10-15 | A2: CI and release workflows written (first run pending). |
| 2026-10-15 | A9 done: commands re-measure only changed texts; bounding-box cache per object. |
| 2026-10-15 | A1 done: shared undo snapshots (0.08 MB for 200 edits on 50,000 objects). |
| 2026-10-15 | B0: fig2dev's options ported (general and PostScript/EPS/PDF/PNG ones; the rest refused with their task); X11 colour names from `colors.c`. |
| 2026-10-14 | B0: fig2dev's export bounds ported; corpus EPS median difference 0.015 → 0.006, 95th percentile 0.072 → 0.048; bounding boxes equal fig2dev's. |
| 2026-10-14 | A10 landed: corpus comparison with fig2dev's EPS and PDF (ghostscript, test-only). Found and fixed: metric text was magnified with the drawing. xfig passes `-F` (true point sizes) to fig2dev for PS/EPS/PDF, so RustFig's 1/72-inch points are right. |
| 2026-10-14 | B0 started: xfig's export-language table (`mode.c`), LaTeX box driver (`genbox.c`), pspdf, `file.export_formats`, language menu in the export dialog, CLI `formats`/`--format`. Found: fig2dev's bounding box (`bound.c`) differs from the ink bounds; porting it is B0's next step. A10 added (EPS/PDF comparison with fig2dev on xfig's sample figures, asked by the user). |
| 2026-10-14 | D9 landed: fig2dev's arrowheads, pie-arc arrows, arc direction flag, PostScript dash patterns. Decision: PostScript (`genps.c`) is the reference where fig2dev's drivers disagree. |
| 2026-10-14 | A8 landed (fig2dev reference tests, no ghostscript). Decision: fig2dev is ported to Rust, no runtime fig2dev or ghostscript: B0 rewritten, B6 without ghostscript, B9 added (remaining drivers), D9 widened by A8's findings. |
| 2026-10-14 | Decision: preferences are stored as TOML (E1). |
| 2026-10-14 | Decisions: every xfig function, modern printing (B5), bundled libraries (D6), preferences (E1) and xfig options (S3) without user-specific defaults; D11 added; outlook: WYSIWYG LaTeX. |
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
