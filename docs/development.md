# Development

**Status:** 2026-10-16 (B12: exports through fig2dev; corpus lists).

## Build and run

```sh
cargo run --release -p rustfig -- [--control 7979] [file.fig]
cargo run --release -p rustfig-cli -- convert in.fig out.pdf
cargo xtask ci        # fmt, clippy -D warnings, tests, layers, assets, licences, wasm
UPDATE_REFERENCE=1 cargo test -p rustfig-engine --test fig2dev_reference -- --nocapture
                      # re-record the fig2dev comparison scores (needs fig2dev;
                      # set RUSTFIG_FIG2DEV if it isn't on PATH)
cargo test --release -p rustfig-engine --test fig2dev_corpus -- --ignored --nocapture
                      # compare EPS/PDF/PNG with fig2dev on xfig's sample
                      # figures (needs fig2dev and ghostscript; RUSTFIG_GS)
RUSTFIG_CORPUS_LIST=plan/corpus-sample.txt cargo test --release -p rustfig-engine \
  --test fig2dev_corpus -- --ignored --nocapture
                      # only the figures listed in FILE (repo-relative paths,
                      # one per line, `#` starts a comment); the local sample list
                      # (plan/ is gitignored) is 150 random figures (seed 20261016)
                      # + the 150 largest
```

Every export goes through fig2dev (plan B12): the engine writes the drawing
as Fig text and runs `fig2dev::convert` (`crates/engine/src/fig2dev/`),
so a change to an export lives in the fig2dev repository (`crates/fig2dev` there: the scene writers in
`src/scene/`, the drivers in `src/drivers/`), which keeps its crates free of every RustFig crate.

If `cargo` cannot write to `~/.cargo` (sandboxed agents), set
`CARGO_HOME=$PWD/.cargo-home` (it's gitignored).

## Quality gates

`cargo xtask ci` must pass before every commit. `cargo xtask wasm` needs the
`wasm32-unknown-unknown` target. When the target is missing the check is
skipped with a warning; set `RUSTFIG_REQUIRE_WASM=1` (as CI does) to make
it fail instead.

`cargo xtask source-audit` (not part of `ci`) lists every function of `old/xfig-3.2.9a/src/*.c` into `docs/source-audit.md`, keeping the status, note and task columns by file and name; run it after porting a function and set the row's status; without `old/` it skips.

GitHub Actions (plan A2): `.github/workflows/ci.yml` runs `cargo xtask ci`
and the perf budgets on Linux for every push and pull request, with fig2dev
installed so the A8 reference tests run. `release.yml` runs on `v*` tags (or
by hand): the gates on Linux, Windows and macOS, then release builds of
`rustfig` and `rustfig-cli` attached to the GitHub release. With rustup: `rustup target add wasm32-unknown-unknown`.

Use rustup rather than Homebrew's `rust` formula: Homebrew's compiler can't
load the official wasm32 standard library, so the target can't be added to
it. On macOS, `brew install rustup` (keg-only; put
`$(brew --prefix rustup)/bin` on your `PATH`), then
`rustup default stable` and `rustup target add wasm32-unknown-unknown`.

xtask runs its cargo commands with the rustc of the toolchain that started
it, so `cargo +toolchain xtask …` stays on that toolchain.

## Fonts

Fonts are never committed here. They live in
[storytold/craft-fonts](https://github.com/storytold/craft-fonts). Build with
`CRAFT_FONTS_DIR="$PWD/../craft-fonts" cargo xtask ci` (absolute path) to embed
them; figbase's `build.rs` (fig2dev repo) generates `figbase::text::CRAFT_FONTS`. Without
it RustFig uses Ubuntu Light and Hack from the `epaint_default_fonts` crate,
with synthetic bold and italic. Everything builds and tests with
`CRAFT_FONTS` empty.

## Driving the app

Start with `--control PORT` and send JSON lines to `127.0.0.1:PORT`:

```json
{"id":1,"method":"engine.execute","params":{"command":"shape.rectangle","params":{"x":10,"y":10,"width":1200,"height":600}}}
{"id":2,"method":"ui.screenshot","params":{"path":"/tmp/shot.png"}}
```

Methods: `engine.execute`, `engine.commands`, `ui.run`, `ui.screenshot`,
`ui.state`, `app.ping` (see `crates/ui-egui/src/control.rs`). Drive tools
with `tool.<name>` followed by `tool.event` (`press`, `move`, `release`,
`double_click`, `key`, `text`; coordinates in Fig units).

## Performance

`rustfig-cli perf` checks budgets on a synthetic 50 000-path document;
`rustfig-cli bench FILE` times one file. Run them before and after renderer,
format or geometry changes, on an idle machine.

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First version. |
| 2026-10-16 | B12: exports through fig2dev; `RUSTFIG_CORPUS_LIST`. |
