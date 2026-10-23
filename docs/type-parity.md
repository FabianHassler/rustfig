# Type (text and fonts) parity checklist

**Status:** 2026-10-23 (E5: character map).

| Item | State |
|---|---|
| 35 PostScript fonts selectable | ✅ |
| 6 LaTeX fonts selectable | ✅ |
| On-screen glyphs | 🟡 craft-fonts faces when built with `CRAFT_FONTS_DIR`, otherwise Ubuntu Light / Hack with synthetic bold and italic |
| Text in PDF | ✅ standard-14 fonts (WinAnsi) |
| Text in EPS | ✅ PostScript fonts re-encoded ISO-Latin-1 |
| Text in SVG | ✅ CSS font families |
| Left/centre/right justification, rotation | ✅ |
| Multi-line text | ✅ |
| Hidden text | ✅ shown as `<<>>` on screen |
| Rigid text | ✅ not scaled with compounds |
| Special (LaTeX) text | 🟡 stored; no LaTeX export yet |
| Character map | ✅ (E5: only characters the font can draw, each shown in that font) |
| Kerning | 🟡 `kern` table only |

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First checklist. |
| 2026-10-23 | E5: character map done; lists only characters the font can draw, in that font. |
