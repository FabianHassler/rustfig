# Effects parity checklist (fills, lines, arrows)

**Status:** 2026-10-14 (D9: arrows, arcs and dashes ported from fig2dev).

| Item | State |
|---|---|
| Area fill shades 0–20, tints 21–40 | ✅ |
| Patterns 41–62 | 🟡 generated in code; in exports they differ from fig2dev's PostScript patterns (D12) |
| Line styles: solid, dashed, dotted, dash-dot, dash-double-dot, dash-triple-dot | ✅ fig2dev's PostScript patterns (`genps.c` `set_style`) |
| Line widths | ✅ fig2dev's `linewidth_adj` |
| Caps, joins | ✅ |
| Arrow types 0–14 | ✅ ported from fig2dev `bound.c`; checked by the A8 reference test |
| Arrows on pie-wedge arcs | ✅ |
| Hollow/filled arrows | ✅ |
| GIF transparent colour | ❌ |

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First checklist. |
| 2026-10-14 | D9: all arrow types, pie-arc arrows, dash patterns and line widths from fig2dev. |
