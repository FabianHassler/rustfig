# RustFig — instructions for agents

RustFig is an open-source, Rust-native reimplementation of xfig: close to the original in look and behaviour, with modern alternatives where xfig's way no longer works (printing, for example). It runs natively on macOS, Windows and Linux, and on the web via WASM (the wasm build compiles; packaging and browser file I/O not yet done).

## Start every session here

1. Read [`ROADMAP.md`](ROADMAP.md) (stage, numbers by dimension, what's next), then [`docs/plan.md`](docs/plan.md) (phased tasks with ids and acceptance criteria) and [`docs/gaps.md`](docs/gaps.md) (every shortfall, ranked). Unless the user gives you a task, take the next task from `docs/plan.md`.
2. Read `plan/STATUS.md` (local session notes, gitignored, may lag) and the relevant section of [`docs/architecture.md`](docs/architecture.md).
3. Work in this loop: orient → plan → implement + test → verify (CI gates, run the app, screenshot) → record (progress docs, `plan/STATUS.md`) → commit. Don't stop to ask unless `docs/plan.md` › "Decisions that are the user's" lists the decision.

## Using xfig's sources (decided 2026-10-14)

- xfig and fig2dev are under the xfig licence (SPDX `Xfig`): use, modify, sublicense and redistribute freely, as long as the copyright and permission notice stay intact. RustFig may read, port and reuse their code, comments and data tables.
- References in `old/` (gitignored, never commit it): the C sources (`old/xfig-3.2.9a/src/`, `old/fig2dev-3.2.9a/fig2dev/`), the format specs (`doc/FORMAT3.2` and older), the HTML manual with screenshots of every panel (`doc/html/`, `doc/html/images/`), `xfig.man.in`, the app defaults (`app-defaults/`), and fig2dev's man pages. Before building UI, look at the manual's image of that panel; with the user's permission, compare with the real xfig (`~/bin/xfig`, XQuartz).
- Ported code: translate into idiomatic Rust that follows this file's rules (no panics, `Result`, layering) rather than transliterating C. Put a comment at the top of the Rust item or file naming the source, e.g. `// Ported from xfig src/u_draw.c (xfig licence, see LICENSE-xfig).` Copyright holders not yet listed go into `LICENSE-xfig`.
- Some files under `old/` are not under the xfig licence (fig2dev's `lib/` replacement C functions; several figure libraries in `Libraries/` carry their own notice or none). Check the header before reusing; library figures follow the asset rules below.
- Algorithms from papers are fine; cite them in `NOTICE` (X-splines: Blanc & Schlick 1995).

## Workspace

| Layer | Crate | Path | Role |
|---|---|---|---|
| L0 | `rustfig-geom` | `crates/geom` | points, transforms, X-splines, arcs, arrowheads, hit testing |
| L1 | `rustfig-model` | `crates/model` | document model, styles, colours, object transforms, flattening |
| L1 | `rustfig-text` | `crates/text` | Fig font tables, font selection, metrics, glyph outlines |
| L2 | `rustfig-format` | `crates/format` | Fig 3.0–3.2 read, Fig 3.2 write, native `.rustfig` JSON |
| L3 | `rustfig-render` | `crates/render` | scene → raster (tiny-skia), SVG, EPS, PDF, PNG; fill patterns |
| L4 | `rustfig-engine` | `crates/engine` | commands, undo, selection, indicator, view, tools, guard |
| L5 | `rustfig-mcp` | `crates/mcp` | MCP server over stdio |
| L6 | `rustfig-ui-egui` | `crates/ui-egui` | egui app: canvas, panels, menus, dialogs, control channel |
| L7 | `rustfig`, `rustfig-cli` | `apps/` | desktop/web app; CLI (convert, info, perf, bench, mcp) |
| — | `xtask` | `xtask/` | `cargo xtask ci \| layers \| assets \| wasm` |

Coordinates are Fig units (1200 per inch; metric drawings use 450 per cm), y grows downwards; line thickness is in 1/80 inch (drawn at 1/160 inch, as fig2dev does).

## Non-negotiables

- Every image, icon, font or other asset must be one of:
  - original work created for RustFig by a contributor (who licenses it MIT OR Apache-2.0);
  - OSI open source;
  - public domain / CC0;
  - Creative Commons with redistribution allowed.
- **Every asset file must have a row in [`ASSETS.md`](ASSETS.md)** (path, author, source URL, licence, notes). Put licence texts next to the assets and summarize them in `NOTICE`. There are no asset files today: icons (`crates/ui-egui/src/icons.rs`), fill patterns (`crates/render/src/pattern.rs`) and colour tables are generated in code. Keep it that way where you can.
- `cargo xtask assets` (run by `cargo xtask ci`) fails on any unattributed asset and on any font file.
- **Fonts live in [storytold/craft-fonts](https://github.com/storytold/craft-fonts), never in this repo** (rules: craftrules [`standards/fonts.md`](https://github.com/storytold/craftrules/blob/main/standards/fonts.md)). Never commit a font file here; add new fonts to craft-fonts. The app uses them only through the optional `CRAFT_FONTS_DIR` build option (`CRAFT_FONTS_DIR="$PWD/../craft-fonts" cargo xtask ci`; use an absolute path), read by `crates/text/build.rs` into `rustfig_text::CRAFT_FONTS`; never add it to a `Cargo.toml`. Everything must build, test and run with `CRAFT_FONTS` empty (the fallback is Ubuntu Light and Hack from the `epaint_default_fonts` crate, with synthetic bold/italic), and tests of craft-fonts glyphs skip when it is empty.
- Code and original assets are MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`); ported xfig/fig2dev parts also keep the xfig notice (`LICENSE-xfig`). If ArtCraft logos are ever added under `docs/brand/`, they are trademarks, not open source (`docs/brand/LICENSE-brand.txt`): never modify them or use them outside RustFig.
- **Never crash: no panics in shipped code.** A crash loses the user's unsaved work. Files, pasted data, MCP and control messages, and command params are all untrusted input.
  - Fallible work returns `Result` (or `Option`); callers handle it or propagate it with `?`. Failures reach the user as an error message (`EngineError` for commands, `FormatError` in the format crate, `String` elsewhere below the engine), never as an abort.
  - Workspace clippy lints ban these outside tests, so `cargo xtask ci` fails on them: `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`. Use instead: `?`, `.ok_or(…)?`, `let … else { return … }`, `if let`, `unwrap_or` / `unwrap_or_default` / `unwrap_or_else`, `.first()` / `.last()` / `.split_last()`, and a real error for a match arm you believe is impossible.
  - Don't panic implicitly either: use `.get(i)` instead of `v[i]` or `&s[a..b]` when the index comes from data. Don't divide integers by something that can be zero, cap sizes and counts read from input (allocations, images, loops; see the `MAX_*` constants), and check that floats are finite before casting them or looping on them.
  - Don't silence errors just to satisfy the lint: use `let _ =` or `.ok()` only where ignoring the failure is the right behaviour, with a comment saying why.
  - Tests may unwrap and panic (`#[test]`, `#[cfg(test)]`, `tests/`). Integration-test files start with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
  - `rustfig_engine::guard` is the last line of defence. Every entry point (commands, tool events, MCP tool calls, the UI frame) catches a panic and reports an internal error; `Engine::execute` also rolls the document back. It exists for bugs and doesn't replace `Result`: wasm aborts on panic, so the web app has no safety net.
  - Untrusted input is fuzzed: `crates/format/tests/prop_format.rs`, `crates/engine/tests/import_fuzz.rs`, `crates/engine/tests/command_sweep.rs` (every command with hostile params; failed mutating commands must leave the document unchanged), `crates/mcp/tests/protocol_props.rs`. Extend these when you add an importer, a parser, a command or a protocol method.
- **Everything is a command.** User-visible behaviour = a command in `crates/engine/src/cmd/*` (id, label, menu path, shortcut, params doc, `mutates`, `enabled`, `run`) + tests. `Engine::execute` makes mutating commands atomic and undoable. Tools (`crates/engine/src/tools/`) keep in-progress state, show it through `Engine::overlay()` and commit by running commands. UI-only commands (dialogs, quit) live in `crates/ui-egui/src/menus.rs` (`UI_COMMANDS`); the menu layout is `LAYOUT` there. The control channel and MCP reach all of them. Command ids and shortcuts must stay unique (tests check).
- **Layering** is enforced by `cargo xtask layers`. A crate depends only on lower layers; nothing below L6 depends on egui/eframe/winit/rfd/epaint.
- **The UI is thin**: panels read engine state and act through `app.run(id, params)`. Colours come from `theme::Tokens`. Window layout: `Layout::Classic` (xfig's arrangement, default) or `Layout::Modern`; the mouse-function indicator reads `Engine::mouse_hints()`, which must describe what the tools really do.
- **UI tests**: `crates/ui-egui/tests/ui_smoke.rs` drives the real `App` headless with `egui_kittest` (`App::with_engine`, `app.ui(ctx)`). File dialogs go through `app.file_dialogs` (`FileDialogs`); tests use `ScriptedDialogs`, never real dialogs. Give custom widgets an accessible label (`widget_info`) so tests can find them.
- **Rust only** (no handwritten JS/TS; `web/index.html` is the trunk shell). **Never break wasm**: native-only code (file dialogs, threads, sockets, `std::time::Instant`) sits behind `cfg(not(target_arch = "wasm32"))`.
- **Quality gates** before every commit: `cargo xtask ci` (fmt, clippy -D warnings including the no-panic lints, tests, layers, assets, wasm). One task id per commit (`A1: shared undo snapshots`), ending with the attribution line the session asks for.

## Environment notes

- If cargo can't write `~/.cargo` (sandboxed agents), use `export CARGO_HOME=$PWD/.cargo-home` (gitignored). Fetching new crates needs network access to crates.io; one crate archive contains `.vscode/settings.json`, which some sandboxes refuse to write.
- Rust comes from rustup (Homebrew's keg-only `rustup` formula, on `PATH` through `~/.zshrc`; Homebrew's `rust` formula is uninstalled): toolchain stable with clippy, rustfmt and `wasm32-unknown-unknown`, so `cargo xtask ci` runs the wasm check. A shell that doesn't read `~/.zshrc` needs `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`. Run the gates with `RUSTFIG_REQUIRE_WASM=1` (as CI must) so a missing target fails instead of skipping.
- Shell gotchas: `mv`/`cp` are aliased interactive (use `/bin/mv -f` / `/bin/cp -f`), and zsh `noclobber` is on (use `>|` to overwrite with a redirect).
- Parallel agents: separate `CARGO_TARGET_DIR` per agent; edit only the crates you own; write manifests atomically. Each target dir grows large: delete yours when you finish (a full disk fails links with `errno=28`).

## Running and looking at the app

- `cargo run --release -p rustfig -- --control 7979 [file.fig|file.rustfig]`. Sibling apps' agents use the same default port: if the status bar or log says the port is unavailable, pick another one, otherwise your requests reach a different app. Launching the GUI and connecting to the port may need the sandbox disabled.
- Drive it with JSON lines on `127.0.0.1:7979`, one reply line per request:
  - `{"id":1,"method":"engine.execute","params":{"command":"shape.rectangle","params":{"x":600,"y":600,"width":1200,"height":600}}}`
  - Tools: `{"command":"tool.box"}`, then `{"command":"tool.event","params":{"type":"press","x":0,"y":0}}` (types `press`, `move`, `release`, `double_click`, `key`, `text`; `"button": "left"|"middle"|"right"`; Fig coordinates). Mouse buttons follow `Engine::mouse_scheme()` (xfig's three buttons by default; `tool.mouse_buttons` switches to the modern scheme).
  - `{"id":2,"method":"ui.screenshot","params":{"path":"/abs/path/shot.png"}}` (write it inside your scratchpad or the project, so you can read it).
  - Other methods: `engine.commands`, `ui.run` (UI commands too), `ui.state`, `app.ping` (`crates/ui-egui/src/control.rs`).
- CLI: `rustfig-cli convert in.fig out.{fig,rustfig,svg,eps,pdf,png}`, `rustfig-cli info FILE`.
- **Performance:** `rustfig-cli perf` checks the budgets (scene build, render fit/pan/zoom, hit test, save/open .fig, SVG export) on a synthetic 50k-path document; `rustfig-cli bench FILE` times one file. Run them before and after renderer, format or geometry changes, on an idle machine (on Linux the report warns when the load average makes timings noisy).
- MCP: `rustfig-cli mcp` (see `docs/mcp.md`).

## Roadmap and progress docs

The progress docs follow craftrules' [`standards/progress-docs.md`](https://github.com/storytold/craftrules/blob/main/standards/progress-docs.md):

- [`ROADMAP.md`](ROADMAP.md): stage, headline numbers, dimensions, features, languages, upcoming, progress log.
- [`docs/plan.md`](docs/plan.md): phased tasks with ids, acceptance criteria and hours; the user's decisions.
- [`docs/target-app-parity.md`](docs/target-app-parity.md): the authoritative parity assessment (area table, methodology, calibration, evidence of what ships).
- [`docs/gaps.md`](docs/gaps.md): every known shortfall, ranked. [`docs/roadmap.md`](docs/roadmap.md): milestones and current focus.
- Checklists: [`ui-parity.md`](docs/ui-parity.md), [`file-format-parity.md`](docs/file-format-parity.md), [`hardware-parity.md`](docs/hardware-parity.md), [`localization-parity.md`](docs/localization-parity.md), [`effects-parity.md`](docs/effects-parity.md), [`type-parity.md`](docs/type-parity.md); [`architecture.md`](docs/architecture.md), [`development.md`](docs/development.md), [`mcp.md`](docs/mcp.md).

When a task lands, update in the same commit: the task row in `docs/plan.md`, the area row in `docs/target-app-parity.md` (score, missing items, hours) and its evidence list, the gap entry in `docs/gaps.md` if it closed or shrank, the checklist it belongs to, the milestone row in `docs/roadmap.md`, and the ROADMAP's tables and progress log when a headline number moves (commands, tests, formats, perf, parity). Update each touched doc's status line and revision history.

- Keep the README's Status section in step with the ROADMAP headline.
