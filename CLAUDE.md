# RustFig — instructions for agents

RustFig is an open-source, Rust-native reimplementation of xfig: close to the original in look and behaviour, with modern alternatives where xfig's way no longer works (printing, for example). It runs natively on macOS, Windows and Linux, and on the web via WASM (the wasm build compiles; packaging and browser file I/O not yet done).

## Start every session here

1. Read [`ROADMAP.md`](ROADMAP.md) (stage, numbers, what's next), then [`docs/plan.md`](docs/plan.md) (phased tasks with ids and acceptance criteria) and [`docs/gaps.md`](docs/gaps.md) (every shortfall, ranked). Unless the user gives you a task, take the next task from `docs/plan.md`.
2. Read `plan/STATUS.md` (local session notes, gitignored, may lag) and the relevant section of [`docs/architecture.md`](docs/architecture.md).
3. Work in this loop: orient → plan → implement + test → verify (CI gates, run the app, screenshot) → record (progress docs, `plan/STATUS.md`) → commit. Don't stop to ask unless `docs/plan.md` › "Decisions that are the user's" lists the decision.

Read when the task needs it:

| File | When |
|---|---|
| [`docs/agents/rules.md`](docs/agents/rules.md) | the full rules behind the non-negotiables below (assets, fonts, licences, no-panic details, fuzz tests, UI tests) |
| [`docs/agents/xfig-sources.md`](docs/agents/xfig-sources.md) | porting or reviewing xfig/fig2dev code; the references in `old/` (C sources, format specs, manual with panel screenshots) |
| [`docs/agents/running.md`](docs/agents/running.md) | running and driving the app (control channel), fig2dev, CLI, MCP, perf, reference and corpus comparisons |
| [`docs/agents/progress-docs.md`](docs/agents/progress-docs.md) | which docs to update when a task lands |

## Workspace

| Layer | Crate | Path | Role |
|---|---|---|---|
| ext | `figbase`, `fig2dev` | [FabianHassler/fig2dev](https://github.com/FabianHassler/fig2dev) (git dependency; `[patch]` to `../fig2dev` for joint work) | fig2dev as its own program: geometry, fonts, colours, reader, drivers, scene, rasteriser |
| L0 | `rustfig-geom` | `crates/geom` | geometry + hit testing |
| L1 | `rustfig-model` | `crates/model` | document model, styles, colours, transforms, flattening |
| L1 | `rustfig-text` | `crates/text` | Fig fonts, metrics, glyph outlines |
| L2 | `rustfig-format` | `crates/format` | Fig read/write, `.rustfig` JSON, bridge model ↔ fig2dev |
| L3 | `rustfig-render` | `crates/render` | document → canvas scene |
| L4 | `rustfig-engine` | `crates/engine` | commands, undo, tools, view, guard, exports, pictures |
| L5 | `rustfig-mcp` | `crates/mcp` | MCP server over stdio |
| L5 | `rustfig-os` | `crates/os` | OS calls (screen colour pick, native print dialog): the only `unsafe` |
| L6 | `rustfig-ui-egui` | `crates/ui-egui` | egui app: canvas, panels, menus, dialogs, control channel |
| L7 | `rustfig-cli` | `apps/cli` | CLI (convert, info, perf, bench, mcp) |
| L8 | `rustfig` | `apps/rustfig` | desktop/web app; also runs as `rustfig-cli` and `fig2dev` |
| — | `xtask` | `xtask/` | `cargo xtask ci \| layers \| assets \| licences \| wasm \| source-audit` |

Coordinates are Fig units (1200 per inch; metric 450 per cm), y grows downwards; line thickness is in 1/80 inch, drawn as fig2dev draws it (`geom::stroke_width`).

## Non-negotiables (details in `docs/agents/rules.md`)

- **Never crash: no panics in shipped code.** Fallible work returns `Result`/`Option`; files, pasted data, MCP/control messages and command params are untrusted. `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` fail clippy outside tests; no data-driven `v[i]`/`&s[a..b]`, no division by a possible zero, cap sizes read from input, check floats are finite. Extend the fuzz tests when you add an importer, parser, command or protocol method.
- **No `unsafe`** except per-function in `crates/os`, each with a `// SAFETY:` comment.
- **Everything is a command** (`crates/engine/src/cmd/*`, with tests); tools commit by running commands; UI-only commands in `crates/ui-egui/src/menus.rs`. Ids and shortcuts unique.
- **Layering** (`cargo xtask layers`): depend only on lower layers; nothing below L6 uses egui/eframe/winit/rfd/epaint.
- **Thin UI**: panels read engine state and act through `app.run(id, params)`; colours from `theme::Tokens`; xfig's one window layout; no selection layer. UI tests use `egui_kittest` and `ScriptedDialogs`, never real dialogs.
- **Rust only; never break wasm**: native-only code behind `cfg(not(target_arch = "wasm32"))`.
- **Assets**: every asset has a row in [`ASSETS.md`](ASSETS.md) and an allowed licence; never commit a font file (the one exception is `assets/fonts/rustfig-key-symbols.ttf`). Everything must work without installed fonts; tests that depend on glyphs use `FIG2DEV_BUILTIN_FONTS=1`.
- **Licences**: code is MIT OR Apache-2.0; ported xfig/fig2dev code carries a "Ported from" comment and a `LICENSE-xfig` entry (`cargo xtask licences` checks).
- **Quality gates** before every commit: `cargo xtask ci` with `RUSTFIG_REQUIRE_WASM=1` (fmt, clippy -D warnings, tests, layers, assets, licences, wasm). One task id per commit (`A1: shared undo snapshots`), ending with the attribution line the session asks for. Update the progress docs in the same commit ([`progress-docs.md`](docs/agents/progress-docs.md)).

## Environment notes

- If cargo can't write `~/.cargo` (sandboxed agents), use `export CARGO_HOME=$PWD/.cargo-home` (gitignored). Fetching new crates needs network access to crates.io; one crate archive contains `.vscode/settings.json`, which some sandboxes refuse to write.
- Rust comes from rustup (Homebrew's keg-only `rustup` formula): stable with clippy, rustfmt and `wasm32-unknown-unknown`. A shell that doesn't read `~/.zshrc` needs `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.
- Shell gotchas: `mv`/`cp` are aliased interactive (use `/bin/mv -f` / `/bin/cp -f`), and zsh `noclobber` is on (use `>|` to overwrite with a redirect).
- Parallel agents: separate `CARGO_TARGET_DIR` per agent; edit only the crates you own; write manifests atomically. Delete your target dir when you finish (a full disk fails links with `errno=28`).
- Running the app: always with scratch `RUSTFIG_SETTINGS_DIR` and `RUSTFIG_RECOVERY_DIR` and `--background` (see [`running.md`](docs/agents/running.md)).
