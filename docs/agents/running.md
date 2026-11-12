# Running, driving and measuring RustFig

For tasks that need the running app, fig2dev, the CLI, performance or the reference comparisons (linked from `CLAUDE.md`).

## The app and its control channel

- `RUSTFIG_SETTINGS_DIR=<scratch>/settings RUSTFIG_RECOVERY_DIR=<scratch>/rec cargo run --release -p rustfig -- --control 7979 --background [file.fig|file.rustfig]`. The two directories keep a test run out of your own preferences and recovery folder; `--background` opens the window behind the others without taking the focus (macOS: no Dock icon); `ui.screenshot` still works. If the status bar or log says the port is unavailable, pick another one.
- Drive it with JSON lines on `127.0.0.1:7979`, one reply line per request:
  - `{"id":1,"method":"engine.execute","params":{"command":"shape.rectangle","params":{"x":600,"y":600,"width":1200,"height":600}}}`
  - Tools: `{"command":"tool.box"}`, then `{"command":"tool.event","params":{"type":"press","x":0,"y":0}}` (types `press`, `move`, `release`, `double_click`, `key`, `text`; `"button": "left"|"middle"|"right"`; Fig coordinates). Mouse buttons are xfig's three, per mode (Option/Alt+click is the middle button, macOS Ctrl+click the right, left+right together the middle on Linux and Windows).
  - `{"id":2,"method":"ui.screenshot","params":{"path":"/abs/path/shot.png"}}` (an absolute path).
  - Mouse and keys as a user gives them (queued, one step per frame, through `raw_input_hook` like real events; the reply comes once they are processed): `{"method":"ui.pointer","params":{"action":"click","x":26,"y":12,"button":"left","modifiers":["shift"]}}` (`action` move, press, release, click, double_click, scroll with `dx`/`dy`; window points; modifiers shift, ctrl, alt, command; Alt+left is middle as for real input), `{"method":"ui.modifiers","params":{"modifiers":["shift"]}}` (holds modifiers across frames until changed, `[]` releases; a step's own modifiers add to them), `{"method":"ui.key","params":{"key":"Escape","modifiers":["alt"],"text":"abc"}}` (egui key names; `text` types characters), `{"method":"ui.click_label","params":{"label":"File","button":"left","index":0}}` (clicks the centre of the widget with that accessible label: exact match first, then substring). `{"method":"ui.file_answers","params":{"answers":["/abs/out.pdf"]}}` answers the next open/save/export file dialogs with these paths instead of the system's dialog (scripts, `cargo xtask movie`).
  - `ui.state` also lists `open` (`popup`, `menu`, `dialogs`, `windows`) and `widgets` (label, role, rect in points, disabled) of the last frame. Menus fade in: wait about 0.5 s before `ui.screenshot` of a pop-up.
  - Other methods: `engine.commands`, `ui.run` (UI commands too), `ui.state`, `app.ping` (`crates/ui-egui/src/control.rs`).

## fig2dev, CLI, MCP

- `fig2dev` (own repository, `apps/fig2dev` there): fig2dev 3.2.9a's command line on the ported drivers (`cargo run -p fig2dev-rs -- -L eps in.fig out.eps` in the fig2dev clone). The same command line runs from RustFig: `cargo run -p rustfig-cli -- fig2dev -L eps in.fig out.eps` (or the `rustfig` binary under the name `fig2dev`). Its options follow `fig2dev.c` and the drivers' sources, not the man page. fig2dev's own test suite: `FIG2DEV_SOURCES=$PWD/old cargo test -p fig2dev-rs --test fig2dev_suite -- --ignored --nocapture` in the fig2dev clone (report in its `target/fig2dev-suite.txt`).
- CLI: `rustfig-cli convert in.fig out.{fig,rustfig,svg,eps,pdf,png,box} [--format LANG] [--option KEY=VALUE]…` (fig2dev's options by name or letter), `rustfig-cli formats [LANG]`, `rustfig-cli info FILE`.
- MCP: `rustfig-cli mcp` (see [`docs/mcp.md`](../mcp.md)).

## Performance

`rustfig-cli perf` checks the budgets (scene build, render fit/pan/zoom, hit test, undo, save/open .fig, SVG export, grouping everything, dragging the group in Move mode) on a synthetic 50k-path document; `rustfig-cli bench FILE` times one file. Run them before and after renderer, format or geometry changes, on an idle machine (on Linux the report warns when the load average makes timings noisy).

## Comparisons with fig2dev

- **fig2dev reference tests** (`crates/engine/tests/fig2dev_reference.rs`): compare RustFig's rendering of `crates/engine/tests/fixtures/*.fig` with fig2dev's (its SVG, rasterised with resvg; skipped without fig2dev). Review `target/fig2dev-reference/<fixture>/diff.png`; after an intended improvement, rerun with `UPDATE_REFERENCE=1` to record the new scores. RustFig never runs the C fig2dev: its drivers are ported to Rust (plan B0). The only external program is ghostscript, optional, for EPS pictures (canvas, PDF and bitmap exports; B15).
- **Corpus comparison** (A10): `cargo test --release -p rustfig-engine --test fig2dev_corpus -- --ignored --nocapture` compares RustFig's EPS, PDF and PNG with fig2dev's (run with xfig's `-F`, rasterised by ghostscript) on xfig's ≈2,550 sample figures; read `target/fig2dev-reference/corpus/index.html` (worst first). Narrow it with `RUSTFIG_CORPUS_FILTER=Flags`, `RUSTFIG_CORPUS_LIMIT=100` or `RUSTFIG_CORPUS_LIST=FILE` (repo-relative paths, `#` comments; the local, gitignored `plan/corpus-sample.txt` holds 150 random figures, seed 20261016, plus the 150 largest, for quick before/after runs). Run it before and after changing a renderer or an exporter; `UPDATE_REFERENCE=1` records an intended improvement. Where fig2dev's drivers disagree, follow its PostScript driver.

- `cargo xtask movie` rebuilds the manual's window pictures (R8) and the inclined-plane movie (R9: `docs/images/movie/`, checked against `examples/inclined-plane/`); `MOVIE_PART=pictures|film` runs one part. Needs ImageMagick, ffmpeg and a TeX installation; takes about 5 minutes.
