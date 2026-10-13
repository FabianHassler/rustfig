# Hardware and platform parity checklist

**Status:** 2026-10-13.

| Item | State |
|---|---|
| macOS (Apple Silicon) | ✅ built and driven via the control channel |
| Windows | 🟡 should build (eframe/glow, rfd); unverified |
| Linux (X11, Wayland) | 🟡 should build; unverified |
| Web (wasm32) | 🟡 code is cfg-gated; build unverified (gap #2) |
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
| 2026-10-13 | First checklist. |
