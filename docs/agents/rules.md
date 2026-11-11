# Rules in detail

The full form of the non-negotiables summarized in `CLAUDE.md`.

## Assets and fonts

- Every image, icon, font or other asset must be one of:
  - original work created for RustFig by a contributor (who licenses it MIT OR Apache-2.0);
  - OSI open source;
  - public domain / CC0;
  - Creative Commons with redistribution allowed.
- **Every asset file must have a row in [`ASSETS.md`](../../ASSETS.md)** (path, author, source URL, licence, notes). Put licence texts next to the assets and summarize them in `NOTICE`. There are no asset files today: mode and attribute icons (xfig's bitmaps as byte arrays in `crates/ui-egui/src/xfig_icons.rs`), fill patterns (fig2dev repo, `crates/fig2dev/src/scene/pattern.rs`) and colour tables are generated in code. Keep it that way where you can.
- `cargo xtask assets` (run by `cargo xtask ci`) fails on any unattributed asset and on any font file.
- **Fonts: RustFig ships none** (decided 2026-10-22), except `assets/fonts/rustfig-key-symbols.ttf`, a 2 KB OFL-1.1 subset of Noto fonts with the key symbols ⌘⇧⌥⌃⌫⎋↩⇥, egui's last fallback font; `cargo xtask assets` allows exactly that path. Never commit another font file. Text in drawings uses the fonts installed on the system, found by figbase (fig2dev repo, `fontdb`: each of xfig's 35 PostScript fonts has candidate families, then the nearest serif/sans/mono), then the TeX Gyre fonts of a TeX installation or the ones "Install TeX Gyre fonts…" downloads from CTAN into the user data folder (`crates/ui-egui/src/tex_gyre.rs`, pinned SHA-256, registered with `add_font_dirs`), then the built-in Ubuntu Light and Hack (`epaint_default_fonts`, with synthetic bold/italic; always on wasm). `FIG2DEV_BUILTIN_FONTS=1` or `rustfig_text::set_builtin_fonts(true)` forces the built-in fonts: tests that depend on glyphs or metrics, the fig2dev reference and corpus comparisons and `perf`/`bench` use it (`cargo xtask ci` sets the variable for the tests), so results do not depend on installed fonts. Everything must build, test and run without any installed font.

## Licences

- Code and original assets are MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`); ported xfig/fig2dev parts also keep the xfig notice (`LICENSE-xfig`, see [`xfig-sources.md`](xfig-sources.md)). If ArtCraft logos are ever added under `docs/brand/`, they are trademarks, not open source (`docs/brand/LICENSE-brand.txt`): never modify them or use them outside RustFig.

## `unsafe`

- Workspace `unsafe_code = "forbid"`, with one exception (decided 2026-10-23: screenpick grew into the one OS-calls crate): `crates/os` (`rustfig-os`: screen colour picking and the native print dialog), `unsafe` denied crate-wide and allowed per OS-call function, each with a `// SAFETY:` comment; safe API, no egui.

## Never crash: no panics in shipped code

A crash loses the user's unsaved work. Files, pasted data, MCP and control messages, and command params are all untrusted input.

- Fallible work returns `Result` (or `Option`); callers handle it or propagate it with `?`. Failures reach the user as an error message (`EngineError` for commands, `FormatError` in the format crate, `String` elsewhere below the engine), never as an abort.
- Workspace clippy lints ban these outside tests, so `cargo xtask ci` fails on them: `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`. Use instead: `?`, `.ok_or(…)?`, `let … else { return … }`, `if let`, `unwrap_or` / `unwrap_or_default` / `unwrap_or_else`, `.first()` / `.last()` / `.split_last()`, and a real error for a match arm you believe is impossible.
- Don't panic implicitly either: use `.get(i)` instead of `v[i]` or `&s[a..b]` when the index comes from data. Don't divide integers by something that can be zero, cap sizes and counts read from input (allocations, images, loops; see the `MAX_*` constants), and check that floats are finite before casting them or looping on them.
- Don't silence errors just to satisfy the lint: use `let _ =` or `.ok()` only where ignoring the failure is the right behaviour, with a comment saying why.
- Tests may unwrap and panic (`#[test]`, `#[cfg(test)]`, `tests/`). Integration-test files start with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
- `rustfig_engine::guard` is the last line of defence. Every entry point (commands, tool events, MCP tool calls, the UI frame) catches a panic and reports an internal error; `Engine::execute` also rolls the document back. It exists for bugs and doesn't replace `Result`: wasm aborts on panic, so the web app has no safety net.
- Untrusted input is fuzzed: `crates/format/tests/prop_format.rs`, `crates/engine/tests/import_fuzz.rs`, `crates/engine/tests/command_sweep.rs` (every command with hostile params; failed mutating commands must leave the document unchanged), `crates/mcp/tests/protocol_props.rs`. Extend these when you add an importer, a parser, a command or a protocol method.

## Everything is a command

User-visible behaviour = a command in `crates/engine/src/cmd/*` (id, label, menu path, shortcut, params doc, `mutates`, `enabled`, `run`) + tests. `Engine::execute` makes mutating commands atomic and undoable. Tools (`crates/engine/src/tools/`) keep in-progress state, show it through `Engine::overlay()` and commit by running commands. UI-only commands (dialogs, quit) live in `crates/ui-egui/src/menus.rs` (`UI_COMMANDS`); the menu layout is `LAYOUT` there. The control channel and MCP reach all of them. Command ids and shortcuts must stay unique (tests check).

## The UI

- **Thin**: panels read engine state and act through `app.run(id, params)`. Colours come from `theme::Tokens`. One window layout, xfig's arrangement (menus File, Edit, View, Snap, Help; file name, message panel, mouse indicator and units box at the top; RustFig's few extra commands in the Edit and View menus). No selection layer (decided 2026-10-22): modes act on the clicked object as in xfig; commands that take ids stay for scripts and MCP; the mouse-function indicator reads `Engine::mouse_hints()`, which must describe what the tools really do.
- **UI tests**: `crates/ui-egui/tests/ui_smoke.rs` drives the real `App` headless with `egui_kittest` (`App::with_engine`, `app.ui(ctx)`). File dialogs go through `app.file_dialogs` (`FileDialogs`); tests use `ScriptedDialogs`, never real dialogs. Give custom widgets an accessible label (`widget_info`) so tests can find them.
