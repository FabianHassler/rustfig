# Development

**Status:** 2026-10-13.

## Build and run

```sh
cargo run --release -p rustfig -- [--control 7979] [file.fig]
cargo run --release -p rustfig-cli -- convert in.fig out.pdf
cargo xtask ci        # fmt, clippy -D warnings, tests, layers, assets, wasm
UPDATE_REFERENCE=1 cargo test -p rustfig-engine --test fig2dev_reference -- --nocapture
                      # re-record the fig2dev comparison scores (needs fig2dev;
                      # set RUSTFIG_FIG2DEV if it isn't on PATH)
cargo test --release -p rustfig-engine --test fig2dev_corpus -- --ignored --nocapture
                      # compare EPS/PDF/PNG with fig2dev on xfig's sample
                      # figures (needs fig2dev and ghostscript; RUSTFIG_GS)
```

If `cargo` cannot write to `~/.cargo` (sandboxed agents), set
`CARGO_HOME=$PWD/.cargo-home` (it's gitignored).

## Quality gates

`cargo xtask ci` must pass before every commit. `cargo xtask wasm` needs the
`wasm32-unknown-unknown` target. When the target is missing the check is
skipped with a warning; set `RUSTFIG_REQUIRE_WASM=1` (as CI should) to make
it fail instead. With rustup: `rustup target add wasm32-unknown-unknown`.

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
them; `crates/text/build.rs` generates `rustfig_text::CRAFT_FONTS`. Without
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
