# Hardware and platform parity checklist

**Status:** 2026-10-25 (D11 screen capture row: manual check per OS). 2026-10-23 (B5b print rows; P2: macOS checked; Windows, Linux and web rows are manual checks, unchecked below).

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
| File > Capture Screen… (D11): the window minimises, the system selector appears (macOS `screencapture -i`: drag an area or Space for a window, Escape cancels; Linux portal Screenshot; Windows: whole screen, no selector), Save asks for the PNG name, the picture lands in the drawing, the window comes back; the logic after the capture is tested, the OS call is not | [ ] manual (needs Screen Recording permission) | [ ] | [ ] X11 portal | [ ] Wayland portal |
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
| File > Merge picks a `.fig` and adds it as one compound (C2) | [ ] | [ ] | [ ] |
| Picture mode (draw a box): the picker offers a PNG, the picture appears and its panel opens; Relink with the same name works (C2) | [ ] | [ ] | [ ] |
| Library mode: the panel lists the libraries ("libraries.json" loads), icons fill in, a figure can be selected and placed; no 404 for `libraries/…` in the network tab (C2) | [ ] | [ ] | [ ] |
| Draw, wait 5 s, reload: "Restore unsaved drawing" appears and Restore brings the drawing back; after Save and reload there is no offer (C3) | [ ] | [ ] | [ ] |
| Retina: the canvas is sharp; resizing the window works | [ ] | [ ] | [ ] |
| Downloads are not blocked (Safari may ask once) | [ ] | [ ] | [ ] |

## Linux findings (2026-10-25, Debian 13, Xfce on X11, ARM64 VM)

- ✅ Installs from the `.deb` (ARM64 build added to the release); starts, panels, rulers and text fonts right; the control channel works.
- ✅ Save and reopen in a folder and file named with accents and CJK; settings in `~/.config/rustfig/settings.toml`, recovery in `~/.local/share/rustfig/recovery`.
- ✅ Export PDF, SVG, PNG, EPS: valid files (`file` identifies them).
- ❌→fixed: the `.deb` shipped `/usr/bin/fig2dev` and collided with Debian's `fig2dev` package (xfig depends on it); the package no longer installs that link, `install.sh` adds it only where no fig2dev exists.
- ✅ CJK characters in the file-name box showed as boxes; fixed 2026-10-25: the UI appends an installed CJK font (Noto Sans CJK, Source Han, PingFang, YaHei, WenQuanYi, ...) and a symbol font as last fallbacks (`key_font.rs`; no font is bundled).
- ✅ On a 1280×712 screen the last row of mode buttons was cut off; fixed 2026-10-25: the mode panel keeps xfig's button size and scrolls vertically (floating scroll bar, no gap).
- ✅ Checked by the user (2026-10-25, X11): three mouse buttons and the left+right chord with matching hints, xfig's and the Ctrl keys, the print dialog (portal), Copy as picture into another program, double-click in Thunar opens the file, screen capture through the portal, crash recovery after `kill -9`.
- Open on Linux: HiDPI scaling, the Wayland session.

## Revision history
| Date | Note |
| 2026-10-25 | First Linux check on Debian 13 (Xfce, X11, ARM64): findings listed above. |
|---|---|
| 2026-10-25 | D11: screen capture row in the per-OS checklist (macOS screencapture, Linux portal, Windows whole screen). |
| 2026-10-23 | C3: browser autosave row in the web checklist. |
| 2026-10-23 | B5b: print dialog rows. |
| 2026-10-23 | P2: per-OS and web checklists; HiDPI and three-button rows updated. |
| 2026-10-23 | C2: browser open, drop, save and export. |
| 2026-10-23 | C1: web build packaged with trunk. |
| 2026-10-13 | First checklist. |
