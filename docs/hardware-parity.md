# Hardware and platform parity checklist

**Status:** 2026-10-22 (Windows, Linux, macOS rows: P1 CI matrix).

| Item | State |
|---|---|
| macOS (Apple Silicon) | ✅ built and driven via the control channel |
| Windows | 🟡 should build (eframe/glow, rfd); CI job added (P1), first run pending |
| Linux (X11, Wayland) | 🟡 should build; unverified |
| Web (wasm32) | 🟡 `cargo xtask web` builds `dist/` (C1); not yet opened in Chrome, Firefox, Safari (see the web checklist) |
| HiDPI / Retina | ✅ canvas renders at physical pixels |
| Mouse: left/right buttons, double click | ✅ |
| Middle-button and space-drag panning, scroll panning | ✅ |
| Pinch / Ctrl-scroll zoom | ✅ |
| Three-button xfig semantics (middle = finish) | 🟡 right click or double click finishes |
| Tablet pressure | ❌ (not an xfig feature) |
| Touch | 🟡 via egui pointer emulation, untested |

## Revision history
| Date | Note |
|---|---|
| 2026-10-23 | C2: browser open, drop, save and export. |
| 2026-10-23 | C1: web build packaged with trunk. |
| 2026-10-13 | First checklist. |
