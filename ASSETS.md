# Assets

Every image, icon, font or other asset file in this repository needs a row
here. `cargo xtask assets` (part of `cargo xtask ci`) fails on any asset file
without a row, and on any font file at all: fonts live in
[storytold/craft-fonts](https://github.com/storytold/craft-fonts).

| Path | Author | Source | Licence | Notes |
|------|--------|--------|---------|-------|

There are currently **no asset files** in the repository. Everything visual
is generated in code:

- Mode panel icons and attribute pictures: xfig's XBM bitmaps (36×32 mode icons, 32×32 attribute pictures) from xfig's `src/w_icons.c`, kept as byte arrays in `crates/ui-egui/src/xfig_icons.rs` (xfig licence, `LICENSE-xfig`).
- Fill patterns 41–62: `crates/fig2dev/src/scene/pattern.rs`.
- The 32 standard colours: `crates/model/src/color.rs` (values defined by the Fig format).
- Fallback fonts come from the `epaint_default_fonts` crate dependency, not from this repository (see `NOTICE`).
