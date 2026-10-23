# RustFig

An open-source, Rust-native reimplementation of the vector drawing program
[xfig](https://mcj.sourceforge.net/), close to the original in look and
behaviour. It runs natively on macOS, Windows
and Linux, with a web (WASM) build on the way.

## Aim

RustFig keeps what makes xfig good: its **simplicity** and its **full
functionality**. Every xfig mode, button and option stays, and works the way
xfig users expect.

Additions must respect xfig. They fit into xfig's way of working (its modes,
its three mouse buttons, its panels) and don't change what an existing xfig
action does. Departing from xfig needs a good reason, for example printing,
which no longer works the old way on today's systems. A modern look is fine;
a different program is not.

## Status

**Alpha. Overall xfig parity ≈78 %** (see [ROADMAP.md](ROADMAP.md) and the
[parity assessment](docs/target-app-parity.md)).

- Draw: circles, ellipses, approximated/interpolated/X-splines, polylines,
  polygons, boxes, rounded boxes, regular polygons, arcs, text, pictures.
- Edit (xfig's modes, no selection layer): move, copy, delete, scale, rotate,
  flip, edit attributes with live panels, update, add/move/delete points,
  glue/break/open compounds, join/split, chop, trim/split/extend, align,
  convert, open/close, arrows, tangent/normal, measure, multi-level undo.
- Settings: preferences, xfig's command line, styles with families, colours
  (picker, user colours, pick from screen).
- Files: reads Fig 1.3–3.2, writes Fig 3.2 and a native JSON format.
- Export (every format through fig2dev's drivers, ported to Rust): PDF, SVG, EPS, PNG, GIF, JPEG, TIFF, PPM, PCX, XBM, XPM, LaTeX box (all 22 xfig fill patterns, arrowheads and dashes as fig2dev draws them); from fig2dev's ported drivers LaTeX picture, epic/eepic/eepicemu, PiCTeX, TeXtyl, tpic, pic, pstex/pdftex, pict2e, CGM, HPGL, DXF, MetaPost, METAFONT, ge, image map, Gerber, shape, TikZ, PSTricks, EMF, Tk/Perl-Tk and AutoCAD slides (every fig2dev language).
- Automation: every action is a command, reachable from a JSON control
  channel, an MCP server and the command line.

## Installation

There are no installers or prebuilt binaries yet; RustFig is built from source
with Rust's own tools.

1. **Rust 1.92 or newer** from [rustup](https://rustup.rs) (`rustup update`
   brings an older one up to date).
2. **Linux only:** the libraries the window system needs, e.g. on
   Debian/Ubuntu:

   ```sh
   sudo apt install pkg-config libgtk-3-dev libxkbcommon-dev libwayland-dev \
     libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
   ```

   macOS and Windows need nothing extra.
3. **Install** both programs (`rustfig`, the drawing program, and
   `rustfig-cli`, the command line):

   ```sh
   git clone https://github.com/FabianHassler/rustfig
   cd rustfig
   cargo install --locked --path apps/rustfig
   cargo install --locked --path apps/cli
   ```

   Or install straight from GitHub, without a clone:

   ```sh
   cargo install --locked --git https://github.com/FabianHassler/rustfig rustfig rustfig-cli
   ```

   (add `--branch`, `--tag` or `--rev` to pick a version; the default is the
   latest `main`).

   The repository is private for now, so both ways need your GitHub login
   (`gh auth login`, or an SSH key; for `cargo install --git` also set
   `CARGO_NET_GIT_FETCH_WITH_CLI=true` so Cargo uses git's login). Cargo
   fetches fig2dev, the converter RustFig exports through, from its own public
   repository ([FabianHassler/fig2dev](https://github.com/FabianHassler/fig2dev))
   by itself.

The programs go into `~/.cargo/bin` (Windows: `%USERPROFILE%\.cargo\bin`),
which rustup puts on your `PATH`. Update later with `git pull` and the same
two `cargo install` lines (or rerun the `--git` line with `--force`); remove with `cargo uninstall rustfig rustfig-cli`.

Optional: `CRAFT_FONTS_DIR=/abs/path/to/craft-fonts cargo install …` embeds the
[craft-fonts](https://github.com/storytold/craft-fonts) collection for closer
PostScript font metrics; without it RustFig uses built-in fallback fonts.

## Quick start

```sh
rustfig drawing.fig                         # draw
rustfig-cli convert drawing.fig drawing.pdf # convert without opening a window
rustfig-cli formats                         # list the export languages
rustfig-cli mcp                             # MCP server for AI assistants
```

From the source tree without installing: `cargo run --release -p rustfig -- drawing.fig`.

Tool shortcuts follow xfig: `C` circle, `E` ellipse, `L` polyline,
`P` polygon, `B` box, `I` spline, `R` arc, `T` text, `M` move, `D` delete,
`S` select. Left click adds points; double click, right click or Enter
finishes; Esc cancels. Without a three-button mouse: Option/Alt + click is the
middle button, and on macOS Ctrl + click the right one (on Linux and Windows,
left and right pressed together also give the middle button).

## fig2dev

RustFig includes [fig2dev](https://github.com/FabianHassler/fig2dev), xfig's
converter, ported to Rust: the same command line, options, messages and exit
codes as fig2dev 3.2.9a, without ghostscript or other helper programs. Use it
three ways:

```sh
rustfig fig2dev -L pdf drawing.fig drawing.pdf     # through the rustfig program
rustfig-cli fig2dev -L eps drawing.fig drawing.eps # or rustfig-cli
fig2dev -L svg drawing.fig drawing.svg             # under its own name (see below)
```

Common uses (all of fig2dev's options are accepted; `rustfig fig2dev -h` lists
them, `-h` after `-L LANG` shows the language's own options):

```sh
rustfig fig2dev -L pdf -m 0.5 in.fig out.pdf        # half size
rustfig fig2dev -L png -m 2 -S 4 in.fig out.png     # bitmap, double size, smoothed
rustfig fig2dev -L pdftex in.fig out.pdf            # graphics for LaTeX ...
rustfig fig2dev -L pdftex_t -p out.pdf in.fig out.pdf_t   # ... and the text part
rustfig fig2dev -L tikz in.fig out.tex              # TikZ picture
rustfig fig2dev -L eps -D +10,20 in.fig out.eps     # only depths 10 and 20
cat in.fig | rustfig fig2dev -L svg > out.svg       # stdin to stdout
```

Languages: PostScript, EPS, PDF, SVG, PNG, GIF, JPEG, TIFF, PPM, PCX, XBM,
XPM, LaTeX picture, epic, eepic, eepicemu, PiCTeX, TeXtyl, tpic, pic,
pstex/pdftex/pspdftex, pict2e, TikZ, PSTricks, LaTeX box, CGM, HPGL, DXF,
MetaPost, METAFONT, ge, image map, Gerber, shape, EMF, Tk, Perl-Tk and
AutoCAD slides (`sld`).

**As `fig2dev` for old scripts and Makefiles:** the `rustfig` program behaves
as fig2dev when it is started under that name, so a link is enough:

```sh
ln -s ~/.cargo/bin/rustfig ~/.local/bin/fig2dev   # any directory on PATH
```

On Windows copy `rustfig-cli.exe` to `fig2dev.exe` instead (the GUI program
has no console window for messages). If the C fig2dev is installed too, the one
that comes first on `PATH` runs (`which -a fig2dev` shows both). fig2dev can
also be installed on its own, without RustFig: see
[its README](https://github.com/FabianHassler/fig2dev#installation).

**Pictures inside figures:** PDF pictures stay vectors in PDF, PostScript and
EPS output. EPS pictures are copied into PostScript/EPS output; for PDF and
bitmap output they need ghostscript (`gs`) installed, otherwise their preview
image is used.

## Documentation

[Architecture](docs/architecture.md) · [Development](docs/development.md) ·
[MCP](docs/mcp.md) · [Gaps](docs/gaps.md) · [Milestones](docs/roadmap.md)

## License

Code is licensed under MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Parts ported from xfig and fig2dev also carry the xfig licence notice (`LICENSE-xfig`).
See `NOTICE` and `ASSETS.md`.
