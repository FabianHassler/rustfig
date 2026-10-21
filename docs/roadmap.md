# Milestones

**Status:** 2026-10-21 (S4 done). Earlier: 2026-10-18. Current focus: **M4 (exports)**: phase W, phase A, B1, B2, B3, B6, B10, B12 and G1 done; every fig2dev language is ported except `sld` (B9); next the audit's hardening passes (B11, octal-escape text bytes first) page layout and previews (B4a/B4b) done 2026-10-18. Task-level plan: [plan.md](plan.md).

| Milestone | Content | State |
|-----------|---------|-------|
| M0 | Workspace, lints, xtask gates, layering | done |
| M1 | Geometry (X-splines, arcs, arrows), model, Fig 3.x I/O, native format | done |
| M2 | Renderer: screen, PNG, SVG, EPS, PDF; patterns; text | done |
| M3 | Engine (commands, undo, tools), egui app, control channel, MCP, CLI | done |
| M4 | Exports: LaTeX family, PostScript/print, more bitmaps | in progress (every fig2dev language but `sld` done; page layout, previews, hardening left) |
| M5 | Web build verified and packaged; browser file I/O | planned |
| M6 | Libraries, tangent/normal and measure modes (open compound, join/split, chop done) | planned |
| M7 | Settings, rulers, colour editor, character map | planned |
| M8 | Beta: parity ≥ 85 % | planned |

## Revision history

| Date | Note |
|------|------|
| 2026-10-22 | D3 done: Chop mode (`e_chop.c`), intersections in `geom/intersect.rs`; M6 edit modes now open compound, join/split and chop. |
| 2026-10-21 | W16, W17 done (one mouse scheme, one layout); D1 save query. |
| 2026-10-21 | S4 done: every xfig function audited (`source-audit.md`); S4-1 to S4-7 added to phase S. |
| 2026-10-21 | DD1–DD10 (dedup with fig2dev) done; docs swept (DD9). |
| 2026-10-13 | M0–M3 landed. |
| 2026-10-14 | Phase W: W1 (layout), W2 (mouse buttons) and W3 (depth panel) landed. |
| 2026-10-14 | Phase W done (W4 Snap menu, W5 attribute panel, W6 rulers, W7 accelerators). |
| 2026-10-15 | B0: fig2dev's languages, bounds and options ported; D12 fill patterns. |
| 2026-10-16 | M4 in progress: B2 pstex/pdftex, G1 old Fig files, ported fig2dev languages exported from the app; B11 hardening added. |
| 2026-10-16 | M4: B12 done, fig2dev a program of its own; every export through it. |
| 2026-10-17 | M4: pict2e ported (B3). |
| 2026-10-18 | M4: TikZ, PSTricks, EMF, Tk/Perl-Tk ported (B1, B3, B9); suite 79 pass. |
| 2026-10-21 | M4: B15 done, PDF and EPS picture objects in exports and on the canvas. |
