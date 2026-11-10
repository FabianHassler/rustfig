# Automation

Everything RustFig does is a **command** with an id (for example `shape.rectangle`, `transform.rotate`, `tool.chop`). The menus, keys and mode buttons all run commands, and so can programs.

- **The command list**: Help > RustFig Reference shows every command with its id, parameters and shortcut. It is also the reference for the other routes.
- **Control channel**: start with `rustfig --control 7979 FILE.fig` and send JSON lines to `127.0.0.1:7979`: `{"id":1,"method":"engine.execute","params":{"command":"shape.rectangle","params":{"x":600,"y":600,"width":1200,"height":600}}}`. Other methods list commands, run user-interface commands such as `help.open`, report the window state or take a screenshot. `--background` opens the window without taking the keyboard focus.
- **MCP**: `rustfig-cli mcp` is a Model Context Protocol server over standard input and output, so an AI assistant can draw and edit figures with the same commands, and render the result. Setting it up for Claude Code, Claude Desktop, Codex or Cursor, with examples, is described in [AI assistants](ai-assistants.md).
- **Command line**: `rustfig-cli convert in.fig out.pdf` converts files, `rustfig-cli info FILE` describes one, and `rustfig fig2dev ARGS` (or running the program under the name `fig2dev`) is fig2dev's own command line (`fig2dev -L eps in.fig out.eps`), whose options are documented in fig2dev's manual page; `fig2dev -h` lists the general options, `fig2dev -h -L eps` adds those of one language.

Coordinates in commands are Fig units: 1200 per inch, 450 per centimeter in metric drawings, y growing downwards. Mutating commands are atomic and one undo step each. A parameter a command does not take is not an error: the command runs, and its reply's `warnings` name each parameter it ignored (a misspelt name, or `color` where the command wants `pen_color`).

**Names**: object ids are session numbers and change when a file is opened. Give an object a name with `object.set_attributes {"ids":[7],"name":"legend"}` (1 to 100 characters; `""` removes it): the name is stored in the .fig as a comment line `rustfig-name: legend`, which xfig and fig2dev ignore, and `object.list` shows it. Every command that takes `ids` also takes `"names": ["legend"]` (alone or together with `ids`). Names need not be unique: a name selects every top-level object carrying it, so copies of a named object answer to it too; an unknown name is an error.

To open a manual page from a script: `{"method":"ui.run","params":{"command":"help.open","params":{"topic":"chop.md"}}}`.
