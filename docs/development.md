# Development

**Status:** 2026-10-26 (Homebrew formulae in the tap).

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

`cargo xtask libraries` writes the libraries RustFig makes itself (`libraries/Feynman/*.fig`, plan D13) from `xtask/src/libraries.rs`: edit the generator, rerun it and commit the files (a test fails when they differ from the generator's output).

`cargo xtask source-audit` (not part of `ci`) lists every function of `old/xfig-3.2.9a/src/*.c` into `docs/source-audit.md`, keeping the status, note and task columns by file and name; run it after porting a function and set the row's status; without `old/` it skips.

GitHub Actions (plans A2, P1): `.github/workflows/ci.yml` runs `cargo xtask ci`
on Linux, Windows and macOS (matrix, fail-fast off, one rust-cache per OS). It
runs on every push to main and every pull request (the repository is public
since 2026-10-24), and by hand (`gh workflow run ci.yml`); `cargo xtask ci`
still runs locally before every commit. Linux alone installs the system packages (with
fig2dev, so the A8 reference tests run; the other runners skip them), runs the
wasm check (`RUSTFIG_REQUIRE_WASM=1`) and the perf budgets (tuned there). The
other runners set `RUSTFIG_SKIP_WASM=1`, which makes `cargo xtask ci|wasm` skip
the wasm check. `.gitattributes` checks text out with LF on every platform. `release.yml` runs on `v*` tags (or
by hand): the gates on Linux, Windows and macOS, then release builds of
`rustfig` and `rustfig-cli` attached to the GitHub release. With rustup: `rustup target add wasm32-unknown-unknown`.

Use rustup rather than Homebrew's `rust` formula: Homebrew's compiler can't
load the official wasm32 standard library, so the target can't be added to
it. On macOS, `brew install rustup` (keg-only; put
`$(brew --prefix rustup)/bin` on your `PATH`), then
`rustup default stable` and `rustup target add wasm32-unknown-unknown`.

xtask runs its cargo commands with the rustc of the toolchain that started
it, so `cargo +toolchain xtask …` stays on that toolchain.

## Packaging (P3)

`cargo xtask package` builds `rustfig` in release mode and writes `target/package/` for the host system, without code signing: macOS `RustFig.app` (icon from the app's own drawing through `rustfig --dump-icon` and `iconutil`, `Info.plist` with the `.fig` document type, a `fig2dev` symlink) and `RustFig-VERSION-macos.dmg` (`hdiutil`); Linux `rustfig-VERSION-linux-ARCH.tar.gz` (ARCH x86_64 or arm64; `release.yml` builds both) (binary, `.desktop`, MIME XML for `image/x-xfig`, icon, `install.sh`) and `rustfig_VERSION_ARCH.deb` (needs `dpkg-deb`); Windows `rustfig-VERSION-windows.zip` (exes, `fig-association.reg`, `INSTALL.txt`). All files are generated in `xtask/src/package.rs`. `release.yml` runs it on each OS and uploads the artifacts. `iconutil` and `hdiutil` fail inside some sandboxes. Not done: signing and notarization, an AppImage, a Windows installer.

## Web version on GitHub Pages (C5)

`cargo xtask web` builds the browser version with trunk into `dist/` (`--serve` serves it locally, `--public-url /PATH/` sets the address it is served from). `.github/workflows/pages.yml` builds it with `--public-url /rustfig/` on every release tag (or by hand: `gh workflow run pages.yml`) and publishes it at https://fabianhassler.github.io/rustfig/ (the repository's Pages source is "GitHub Actions").

## Homebrew

The formulae live in the tap [FabianHassler/homebrew-tap](https://github.com/FabianHassler/homebrew-tap) (decided 2026-10-26; the repository's own `Formula/rustfig.rb` is gone): `rustfig` builds `rustfig` and `rustfig-cli` (`--HEAD` until a release tag whose `Cargo.lock` names the current fig2dev commits; then add a stable `url`), `fig2dev-rs` builds fig2dev's `fig2dev` from its tag and conflicts with Homebrew's C `fig2dev`. Bump both when you tag a release.

## Fonts

Fonts are not committed here (the one exception is the key-symbol subset,
see `ASSETS.md`). Text uses the fonts installed on the system, found by
figbase; TeX Gyre fonts from a TeX installation or from Help > Install TeX
Gyre fonts… are used too; the fallback is Ubuntu Light and Hack from the
`epaint_default_fonts` crate with synthetic bold and italic.
`FIG2DEV_BUILTIN_FONTS=1` (or `rustfig_text::set_builtin_fonts(true)`) forces
the built-in fonts: `cargo xtask ci` sets it for the tests, and the fig2dev
reference and corpus tests and `rustfig-cli perf`/`bench` set it themselves.
Everything builds and tests with no font installed.

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

## Side-by-side check against the real xfig (W22)

`cargo xtask xfig-compare` runs the real xfig (`/opt/homebrew/bin/xfig`, default options, `HOME` in a scratch folder) headless under `Xvfb :77` and RustFig (control channel, scratch settings and recovery folders, light theme, zoom 1, Fig (0,0) at the canvas corner) through the same steps for every scenario in the table in `xtask/src/xfig_compare.rs`: each mode selected, one object drawn per drawing mode, the editing modes on a small fixture, the five menus open, the indicator panel's pop-ups. xdotool drives xfig, `xwd | xwdtopnm | pnmtopng` captures it. After each scenario both apps save the `.fig` and the object lines are compared (text height/length ignored: they depend on fonts). Output: `target/xfig-compare/index.html` (window and canvas crops side by side with a difference image, RMSE scores, worst first, the object diff, notes); the reviewed result is `docs/xfig-compare.md`.

Needs `Xvfb`, `xdotool`, `xwd` (XQuartz: `/opt/X11/bin`), `xwdtopnm`, `pnmtopng` (netpbm) and `magick` (ImageMagick); it skips with a message when one is missing. It is not part of `ci`. Narrow it with `XFIG_COMPARE_FILTER=draw-` or `draw-,hint-` (scenario ids contain one of the comma-separated texts), `XFIG_COMPARE_DISPLAY=:77`, `XFIG_COMPARE_PORT=8031`, `XFIG_BIN`. Run it on a machine with no other X server on that display; the RustFig window opens behind others (`--background`) and is as high as the screen allows (the report notes the clipped height).

## Performance

`rustfig-cli perf` checks budgets on a synthetic 50 000-path document;
`rustfig-cli bench FILE` times one file. Run them before and after renderer,
format or geometry changes, on an idle machine.

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First version. |
| 2026-10-16 | B12: exports through fig2dev; `RUSTFIG_CORPUS_LIST`. |
| 2026-10-23 | Homebrew formula. |
| 2026-10-26 | Formulae moved to FabianHassler/homebrew-tap (rustfig, fig2dev-rs). |
| 2026-10-24 | P3: `cargo xtask package`. |
| 2026-10-24 | W22: `cargo xtask xfig-compare`. |
| 2026-10-25 | W22 round 2: `XFIG_COMPARE_FILTER` takes comma-separated substrings; second fixture, `Key`, `Dlg`, `Item`, `Drag` steps. |
