# Assets

Every image, icon, font or other asset file in this repository needs a row
here. `cargo xtask assets` (part of `cargo xtask ci`) fails on any asset file
without a row, and on any font file except the one key-symbol font below.
RustFig otherwise ships no fonts: text uses the system's fonts, the TeX Gyre
fonts a user installs from CTAN (Help menu), and the `epaint_default_fonts`
fallback.

| Path | Author | Source | Licence | Notes |
|------|--------|--------|---------|-------|
| `assets/fonts/rustfig-key-symbols.ttf` | The Noto Project Authors (subset by RustFig) | Noto Sans Symbols 2 (⌘ ⇧ ⌥ ⌫), Noto Sans Symbols (⌃ ⎋), Noto Sans Math (↩ ⇥): `fonts/NotoSansSymbols2`, `NotoSansSymbols`, `NotoSansMath` (`hinted/ttf/*-Regular.ttf`) of https://notofonts.github.io | SIL OFL 1.1 (`assets/fonts/OFL.txt`) | 2 KB, 8 glyphs, family renamed "RustFig Key Symbols"; egui's last fallback font (`crates/ui-egui/src/key_font.rs`). Per source: `pyftsubset <font> --unicodes=… --no-hinting --layout-features= --drop-tables+=DSIG,GSUB,GPOS,GDEF,MATH,BASE,STAT,meta,vhea,vmtx,gasp`, then `fontTools.merge` and a rename (name IDs 1-4, 6). The only font file `cargo xtask assets` allows. |

The only asset file in the repository is the key-symbol font above. Everything visual
is generated in code:

- Mode panel icons and attribute pictures: xfig's XBM bitmaps (36×32 mode icons, 32×32 attribute pictures) from xfig's `src/w_icons.c`, kept as byte arrays in `crates/ui-egui/src/xfig_icons.rs` (xfig licence, `LICENSE-xfig`).
- Fill patterns 41–62: `crates/fig2dev/src/scene/pattern.rs` (in fig2dev's repository).
- The 32 standard colours: `crates/model/src/color.rs` (values defined by the Fig format).
- Fallback fonts come from the `epaint_default_fonts` crate dependency, not from this repository (see `NOTICE`).
