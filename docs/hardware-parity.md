# Hardware and platform parity checklist

**Status:** 2026-10-23 (B5b print rows; P2: macOS checked; Windows, Linux and web rows are manual checks, unchecked below).

| Item | State |
|---|---|
| macOS (Apple Silicon) | ✅ built and driven via the control channel |
| Windows | 🟡 should build (eframe/glow, rfd); CI job added (P1), first run pending |
| Linux (X11, Wayland) | 🟡 should build; unverified |
| Web (wasm32) | 🟡 `cargo xtask web` builds `dist/` (C1); browser print (hidden iframe, B5) unchecked; not yet opened in Chrome, Firefox, Safari (see the web checklist) |
| HiDPI / Retina | ✅ canvas renders at physical pixels, lowered when the GPU's biggest texture is smaller (P2); scales 1 to 4 tested |
| Mouse: left/right buttons, double click | ✅ |
| Middle-button and space-drag panning, scroll panning | ✅ |
| Pinch / Ctrl-scroll zoom | ✅ |
| Three-button xfig semantics (middle = finish) | ✅ left, Alt+click or middle, right; Ctrl+click right on macOS; left+right chord on Linux and Windows (unit and UI tests) |
| Tablet pressure | ❌ (not an xfig feature) |
| Touch | 🟡 via egui pointer emulation, untested |

## Per-OS checklist (P2)

Automated by `cargo xtask ci` on every system: the folders per OS (`crates/engine/src/dirs.rs`), non-ASCII names, HiDPI scales, the three-button mapping. Done on macOS by hand: command-line open of a non-ASCII path, Retina screenshot. The rest needs the real system; tick a box when checked and note the date and version.

| Check | macOS | Windows | Linux X11 | Linux Wayland |
|---|---|---|---|---|
| File > Open and Save As with a name of accents, CJK, emoji and spaces (native dialog; GTK or portal on Linux) | [x] 2026-10-23 scripted dialogs; native dialog [ ] | [ ] | [ ] | [ ] |
| `rustfig "Zeichnung ä 日本.fig"` from a terminal opens it | [x] 2026-10-23 | [ ] | [ ] | [ ] |
| Double-click a `.fig` in the file manager opens it (file association) | [ ] | [ ] | [ ] | [ ] |
| A file name that is not UTF-8 starts without aborting | n/a | n/a | [ ] | [ ] |
| Settings in `~/Library/Application Support/RustFig/settings.toml` (macOS), `%APPDATA%\RustFig\settings.toml` (Windows), `~/.config/rustfig/settings.toml` (Linux) | [x] logic tested | [ ] | [ ] | [ ] |
| Recovery and saved versions in `…/RustFig/recovery` (Windows `%LOCALAPPDATA%`, Linux `~/.local/share/rustfig/recovery`); after `kill -9` the next start offers the drawing | [x] logic tested; kill -9 [ ] | [ ] | [ ] | [ ] |
| Left, middle and right buttons; the mouse indicator matches | [x] | [ ] | [ ] | [ ] |
| Two-button mouse: left+right together is the middle button (Windows, Linux); trackpad: Option+click middle, Ctrl+click right (macOS) | [ ] trackpad | [ ] | [ ] | [ ] |
| HiDPI: sharp at 2x; moving the window between a 1x and a 2x screen redraws sharply | [x] 2x screenshot; mixed screens [ ] | [ ] 125 %, 150 %, 200 % | [ ] | [ ] fractional scale |
| Clipboard: Copy as picture, Paste Text (E14, E7) | [ ] | [ ] | [ ] | [ ] |
| Keyboard: xfig's keys next to the platform's (Cmd or Ctrl), the Alt/Meta menu keys | [ ] | [ ] | [ ] | [ ] |
| File > Print… > Print FIGURE opens the system's print dialog (macOS print panel; Windows print dialog then the PDF viewer's `printto`; Linux portal dialog); Cancel prints nothing (B5b) | [ ] 2026-10-23 Print FIGURE blocks the app in the system panel (not seen: no screen capture permission); printing to paper [ ] | [ ] | [ ] | [ ] |
| Emoji in a file name: the file-name box draws a replacement glyph (no emoji font; known, cosmetic) | seen | [ ] | [ ] | [ ] |

## Web checklist (C1, C2): Chrome, Firefox, Safari

Build with `cargo xtask web`, serve `dist/` (`cd dist && python3 -m http.server 8080`) and open http://127.0.0.1:8080.

| Check | Chrome | Firefox | Safari |
|---|---|---|---|
| The page loads, "Loading RustFig…" goes away, the drawing area and panels appear | [ ] | [ ] | [ ] |
| Draw a box and a polyline; the three buttons (middle: Alt+click; right: Ctrl+click or two-finger click) | [ ] | [ ] | [ ] |
| File > Open shows the picker; a `.fig` opens | [ ] | [ ] | [ ] |
| Dropping a `.fig` on the window opens it | [ ] | [ ] | [ ] |
| Edit, then File > Save: a download named like the file arrives; opening it again gives the edit | [ ] | [ ] | [ ] |
| File > Export (PDF, SVG, EPS): a download arrives and opens in a viewer | [ ] | [ ] | [ ] |
| File > Print… > Print FIGURE opens the browser print dialog for the PDF (hidden iframe; fallback: new tab, pop-ups allowed) (B5b); Print Batch gives one multi-page job | [ ] | [ ] | [ ] |
| Draw, wait 5 s, reload: "Restore unsaved drawing" appears and Restore brings the drawing back; after Save and reload there is no offer (C3) | [ ] | [ ] | [ ] |
| Retina: the canvas is sharp; resizing the window works | [ ] | [ ] | [ ] |
| Downloads are not blocked (Safari may ask once) | [ ] | [ ] | [ ] |

## Revision history
| Date | Note |
|---|---|
| 2026-10-23 | C3: browser autosave row in the web checklist. |
| 2026-10-23 | B5b: print dialog rows. |
| 2026-10-23 | P2: per-OS and web checklists; HiDPI and three-button rows updated. |
| 2026-10-23 | C2: browser open, drop, save and export. |
| 2026-10-23 | C1: web build packaged with trunk. |
| 2026-10-13 | First checklist. |
