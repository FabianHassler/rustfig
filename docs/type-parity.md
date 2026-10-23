# Type (text and fonts) parity checklist

**Status:** 2026-10-23 (F7: installed fonts per text, exports pending; F1: system fonts).

| Item | State |
|---|---|
| 35 PostScript fonts selectable | ✅ |
| 6 LaTeX fonts selectable | ✅ |
| On-screen glyphs | ✅ installed system fonts (Times, Helvetica, Courier, Symbol, …), TeX Gyre from a TeX installation or Help > Install TeX Gyre fonts…, otherwise Ubuntu Light / Hack with synthetic bold and italic (F1) |
| Installed fonts per text (F7) | 🟡 chooser, canvas, `.fig` comment `rustfig-font:` with nearest Fig font as number, missing-font message on open; exports still use the Fig font (embedding is a fig2dev follow-up) |
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
| 2026-10-23 | F1: text uses system fonts and TeX Gyre; craft-fonts removed. |
| 2026-10-23 | E5: character map done; lists only characters the font can draw, in that font. |
| 2026-10-23 | F7: installed fonts per text (chooser, canvas, .fig comment); exports pending. |
