# Science gallery

Sixteen scientific drawings as Fig 3.2 files, generated from the mathematics and physics by `cargo xtask examples [OUTDIR]` (`xtask/src/examples/gallery.rs`; without OUTDIR it rewrites the repository's `examples/`). Original work, MIT OR Apache-2.0.

All text is LaTeX special text (font flags: special bit set, LaTeX default font), written as LaTeX source ($\theta_1$, $pV^\gamma = \mathrm{const}$, $\mu^-$). No Symbol-font characters. On the canvas, View > Typeset LaTeX text shows it typeset.

## How the PDFs are built

For each `X.fig` (build in a temporary directory, the LaTeX aux files stay out of the repository):

```sh
rustfig-cli convert X.fig X.pdf --format pdftex      # writes X.pdf (graphics) and X.pdf_t (LaTeX overlay)
cat > w.tex <<'TEX'
\documentclass[border=6pt]{standalone}
\usepackage{graphicx,color,amsmath,amssymb}
\begin{document}\input{X.pdf_t}\end{document}
TEX
pdflatex -interaction=nonstopmode w.tex             # -> w.pdf = X-figure.pdf
pdftoppm -r 150 -png -singlefile X-figure.pdf X     # preview X.png
```

`X-figure.pdf` is the final typeset drawing, `X.png` its preview, `gallery.pdf` all 16 with a caption each (article class, landscape), `contact-sheet.png` all previews. Label sizes were measured once with pdflatex (`\wd`, `\ht`, `\dp` of each string) and are kept in `xtask/src/examples/metrics.rs`, so bounding boxes and the one label that needs a white box (the angle in double-slit) use the real typeset size and the generator needs no LaTeX; a label not in that table falls back to an estimate from its length (measure it and add it to the table). Each figure carries an invisible white frame rectangle (depth 999) that fixes the cropped page.

## Style

Common style: LaTeX text at 10-12 pt (titles 14 pt), axes 2 wide, a palette of blue, red, green, orange and grays, depths: fills 150, grid 90, curves 60, axes 50, markers 40, labels 30; every logical part is a compound.

## Drawings

| File | Shows | RustFig features |
|---|---|---|
| `pythagoras.fig` | Euclid's figure on a 3-4-5 triangle, altitude splitting the hypotenuse square | compounds, tinted fills, hatching, dashed lines, superscripts |
| `unit-circle.fig` | unit circle with theta, sin, cos, tan segments | circle, arc with arrow, thick segments |
| `riemann.fig` | left Riemann sum, 8 rectangles under f(x) = 1.2 + 2.2 sin(pi x / 4) | X-spline curve, boxes, axes with ticks, dotted guides, depth layering |
| `normal.fig` | normal density with 1, 2, 3 sigma regions (68.27 / 95.45 / 99.73 %) | X-spline, filled polygons in three tints, double-headed arrows |
| `fourier.fig` | square wave and partial sums N = 1, 3, 5, 15 with Gibbs overshoot | many X-splines, colors, legend, leader arrow |
| `complex-plane.fig` | Euler's formula e^(i theta) on the unit circle | arc arrow, dashed projections, display formulas |
| `inclined-plane.fig` | free-body diagram on a 30 degree incline: mg, N, f, components | hatch patterns (44, 45) as a layer under the slope and as ground, arrows with styles, labels on clear white space |
| `pendulum.fig` | pendulum with weight, tension, components, angle | circles, dashed arcs and lines, hatched ceiling, arc with arrow |
| `snell.fig` | refraction air to glass (n = 1.0 to 1.5, 50 deg to 30.7 deg) with reflected ray | tinted media, dashed normal, angle arcs with arrows, mid-line arrowheads |
| `double-slit.fig` | Young's double slit with wavefronts, screen fringes, cos^2 x sinc^2 intensity | about 40 arcs, gray-level fill steps, X-spline plot, hatched barrier |
| `carnot.fig` | Carnot cycle in the p-V plane, gamma = 1.4, shaded work area | X-splines for isotherms and adiabats, direction arrows, indented-butt arrows, dotted guides |
| `rlc.fig` | series RLC circuit with source, current and voltage drops | polyline zigzag, arc coils, plates, double-headed arrows |
| `feynman.fig` | e- e+ to gamma* to mu- mu+ | wavy photon as X-spline, indented arrows, vertices, time arrow |
| `minkowski.fig` | light cone, boosted axes for v = 0.5 c, calibration hyperbolas, one event | dashed lines, X-spline hyperbolas, tinted regions, nested compounds |
| `loglog.fig` | power law on log-log axes with error bars and fit | decade and minor ticks, rotated text, data markers, legend box |
| `damped.fig` | damped oscillator with dashed exponential envelope, T and tau marked | X-splines, dashed lines, rotated axis label, units |
