# RustFig

An open-source, Rust-native reimplementation of the vector drawing program
[xfig](https://mcj.sourceforge.net/), close to the original in look and
behaviour. It runs natively on macOS, Windows
and Linux, with a web (WASM) build on the way.

## Status

**Alpha. Overall xfig parity ≈65 %** (see [ROADMAP.md](ROADMAP.md) and the
[parity assessment](docs/target-app-parity.md)).

- Draw: circles, ellipses, approximated/interpolated/X-splines, polylines,
  polygons, boxes, rounded boxes, regular polygons, arcs, text, pictures.
- Edit: select, move, copy, delete, scale, rotate, flip, edit attributes,
  update, add/move/delete points, glue/break compounds, align, distribute,
  convert line ↔ spline, open/close, arrows, depth ordering, multi-level undo.
- Files: reads Fig 1.3–3.2, writes Fig 3.2 and a native JSON format.
- Export (every format through fig2dev's drivers, ported to Rust): PDF, SVG, EPS, PNG, GIF, JPEG, TIFF, PPM, PCX, XBM, XPM, LaTeX box (all 22 xfig fill patterns, arrowheads and dashes as fig2dev draws them); from fig2dev's ported drivers LaTeX picture, epic/eepic/eepicemu, PiCTeX, TeXtyl, tpic, pic, pstex/pdftex, pict2e, CGM, HPGL, DXF, MetaPost, METAFONT, ge, image map, Gerber, shape, TikZ, PSTricks, EMF, Tk/Perl-Tk (every fig2dev language except `sld`).
- Automation: every action is a command, reachable from a JSON control
  channel, an MCP server and the command line.

## Quick start

```sh
cargo run --release -p rustfig -- drawing.fig
cargo run --release -p rustfig-cli -- convert drawing.fig drawing.pdf
cargo run --release -p rustfig-cli -- mcp
```

Tool shortcuts follow xfig: `C` circle, `E` ellipse, `L` polyline,
`P` polygon, `B` box, `I` spline, `R` arc, `T` text, `M` move, `D` delete,
`S` select. Left click adds points; double click, right click or Enter
finishes; Esc cancels.

fig2dev (the converter RustFig exports through) lives in its own repository, [fahasch/fig2dev](https://github.com/fahasch/fig2dev); until it is published, clone it next to this repository as `../fig2dev` (the root `Cargo.toml` has a `[patch]` for it).

## Documentation

[Architecture](docs/architecture.md) · [Development](docs/development.md) ·
[MCP](docs/mcp.md) · [Gaps](docs/gaps.md) · [Milestones](docs/roadmap.md)

## License

Code is licensed under MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Parts ported from xfig and fig2dev also carry the xfig licence notice (`LICENSE-xfig`).
See `NOTICE` and `ASSETS.md`.
