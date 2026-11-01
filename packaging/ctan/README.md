# RustFig

RustFig is a reimplementation of xfig, the classic vector drawing program
for figures in papers and documents, written in Rust. It reads and writes
xfig's Fig files (Fig 1.3 to 3.2 on reading, Fig 3.2 on writing), so old
figures open unchanged and new ones still work with xfig and fig2dev. It
looks and behaves like xfig, and runs natively on macOS, Windows and Linux.

For TeX users it exports through a Rust port of fig2dev to every one of
fig2dev's languages: PDF, EPS and PostScript, SVG, the combined
pdftex/pstex pairs (graphics plus a LaTeX overlay for the texts), LaTeX
picture, epic/eepic, pict2e, TikZ, PSTricks, PiCTeX, MetaPost, METAFONT and
more, plus Asymptote. Texts marked with the TeX flag are passed to LaTeX
unchanged, and can be shown typeset on the canvas. PDF and SVG files can be
opened or merged as editable objects.

## Contents

- `doc/rustfig-manual.pdf`: the user manual (built from `docs/manual/*.md`);
  also as HTML at https://fabianhassler.github.io/rustfig/manual/index.html.
- The source code: a Cargo workspace (`Cargo.toml`, `Cargo.lock`,
  `crates/`, `apps/`, `xtask/`, `assets/`); the manual's pages in
  `docs/manual/`. The full repository (plans, packaging, web build) is at
  https://github.com/FabianHassler/rustfig.
- `libraries/`: xfig's figure libraries (electronics, Feynman diagrams,
  flowcharts, UML and many more), opened from the Library mode.

## Installing

Ready-made packages for macOS, Windows and Linux, a Homebrew tap, Scoop and
winget manifests are on the project page:
https://github.com/FabianHassler/rustfig/releases

To build from this source you need Rust 1.92 or later (https://rustup.rs):

    cargo build --release --locked -p rustfig -p rustfig-cli

Cargo downloads the dependencies, among them fig2dev's Rust port (crates
`figbase` and `fig2dev-rs-lib`, https://github.com/FabianHassler/fig2dev).
The programs are `target/release/rustfig` (the drawing program) and
`target/release/rustfig-cli` (conversions on the command line, including
fig2dev's own command line as `rustfig-cli fig2dev`).

RustFig ships no fonts: texts use the fonts installed on the system (the
TeX Gyre fonts of a TeX installation if present), with built-in fallbacks.

## Licence

RustFig is free software, licensed under the MIT licence or the Apache
licence 2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The parts
ported from xfig and fig2dev, and xfig's figure libraries, keep the xfig
licence (`LICENSE-xfig`, a permissive notice). See `NOTICE` and `ASSETS.md`
for details.

## Author

Fabian Hassler. Bugs and suggestions: https://github.com/FabianHassler/rustfig/issues
