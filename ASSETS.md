# Assets

Every image, icon, font or other asset file in this repository needs a row
here. `cargo xtask assets` (part of `cargo xtask ci`) fails on any asset file
without a row, and on any font file at all: fonts live in
[storytold/craft-fonts](https://github.com/storytold/craft-fonts).

| Path | Author | Source | Licence | Notes |
|------|--------|--------|---------|-------|

There are currently **no asset files** in the repository. Everything visual
is generated in code:

- Tool icons: `crates/ui-egui/src/icons.rs` (vector drawing with egui).
- Fill patterns 41–62: `crates/render/src/pattern.rs`.
- The 32 standard colours: `crates/model/src/color.rs` (values defined by the Fig format).
- Fallback fonts come from the `epaint_default_fonts` crate dependency, not from this repository (see `NOTICE`).
