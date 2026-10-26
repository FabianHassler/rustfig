# Automation

Everything RustFig does is a **command** with an id (for example `shape.rectangle`, `transform.rotate`, `tool.chop`). The menus, keys and mode buttons all run commands, and so can programs.

- **The command list**: Help > RustFig Reference shows every command with its id, parameters and shortcut. It is also the reference for the other routes.
- **Control channel**: start with `rustfig --control 7979 FILE.fig` and send JSON lines to `127.0.0.1:7979`: `{"id":1,"method":"engine.execute","params":{"command":"shape.rectangle","params":{"x":600,"y":600,"width":1200,"height":600}}}`. Other methods list commands, run user-interface commands such as `help.open`, report the window state or take a screenshot. `--background` opens the window without taking the keyboard focus.
- **MCP**: `rustfig-cli mcp` is a Model Context Protocol server over standard input and output, so an AI assistant can draw and edit figures with the same commands, and render the result.
- **Command line**: `rustfig-cli convert in.fig out.pdf` converts files, `rustfig-cli info FILE` describes one, and `rustfig fig2dev ARGS` (or running the program under the name `fig2dev`) is fig2dev's own command line (`fig2dev -L eps in.fig out.eps`), whose options are documented in fig2dev's manual page; `fig2dev -h` lists the general options, `fig2dev -h -L eps` adds those of one language.

Coordinates in commands are Fig units: 1200 per inch, 450 per centimetre in metric drawings, y growing downwards. Mutating commands are atomic and one undo step each.

To open a manual page from a script: `{"method":"ui.run","params":{"command":"help.open","params":{"topic":"chop.md"}}}`.
