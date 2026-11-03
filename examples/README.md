# Example drawings

Drawings to look at, to learn from and to test RustFig's speed with. Open any of them with File › Open: RustFig reads the compressed `.fig.gz` files directly, and xfig and fig2dev read them too. Each folder holds the drawing, a preview picture and a `SOURCE.txt` that says where it comes from.

## Science gallery

[`gallery/`](gallery/README.md): sixteen drawings from mathematics and physics (Carnot cycle, double slit, Feynman diagram, RLC circuit, Minkowski diagram, …), all text as LaTeX, each with its PDF made by pdflatex. `gallery/gallery.pdf` has all of them; `gallery/contact-sheet.png` shows them side by side.

## Generated from mathematics

Original work for RustFig (MIT OR Apache-2.0), written by `cargo xtask examples`.

| Folder | Drawing | Objects |
|---|---|---|
| `clifford-attractor/` | Clifford attractor: 250,000 dots colored by step direction, on a black page | 250,001 |
| `apollonian-gasket/` | Apollonian gasket (Descartes' circle theorem), circles colored by generation; drawn at twice the size with the file's magnification at 50 %, so the outlines export at 1/320 inch | 4,221 |
| `penrose-tiling/` | Penrose P3 tiling of rhombs after 7 deflations, gold and teal | 7,615 |
| `gosper-curve/` | Gosper curve (flowsnake) of order 5: one continuous line in 142 joined pieces, rainbow colored | 142 (16,808 points) |

## Imported from SVG

RustFig's own conversions (File › Open on the SVG), from public-domain and CC0 sources; `SOURCE.txt` in each folder names the original. Fig 3.2 has no gradients or transparency, so gradients become one color and transparent areas are drawn solid.

| Folder | Drawing | Objects | Source, license |
|---|---|---|---|
| `ne-countries/` | Country borders of the world at 1:10m (Natural Earth) | 4,233 | Natural Earth, public domain |
| `ne-coast/` | Coastlines of the world at 1:10m (Natural Earth) | 4,069 | Natural Earth, public domain |
| `blankmap/` | Blank world map | 2,232 | Wikimedia Commons, public domain |
| `greece/` | Topographic map of Greece with an embedded relief picture | 2,013 | Wikimedia Commons, public domain |
| `cube8/` | Projection of the 8-cube: 190,260 overlapping circles, the heaviest drawing here | 190,260 | Wikimedia Commons, public domain |
| `snowy/` | Snowy landscape | 436 | publicdomainvectors.org via Commons, CC0 1.0 |
| `pines/` | Mountain landscape with pine trees | 31 | Wikimedia Commons, CC0 1.0 |
| `wenig/` | Headpiece with a snowy landscape, by Bernhard Wenig (1897) | 85 | Wikimedia Commons, public domain |

The large ones are the figures of the speed comparison with xfig in [`docs/speed-compare.md`](../docs/speed-compare.md).
