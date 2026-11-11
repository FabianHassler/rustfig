# MCP server

**Status:** 2026-11-06 (setup for Claude Code, Codex and others; the RustFig skill).

`rustfig-cli mcp` speaks the Model Context Protocol (JSON-RPC 2.0, one
message per line) on stdin/stdout. An AI assistant that runs it can draw,
edit, look at, save and export figures with RustFig's commands.

## Setup

1. **Install RustFig** (see the README: Homebrew, the Linux packages, Scoop
   or winget, or a release archive). `rustfig-cli --version` must work in a
   terminal. Clients started from a desktop launcher may not see the shell's
   `PATH`: give them the full path then (`which rustfig-cli`, for example
   `/opt/homebrew/bin/rustfig-cli`; on Windows `rustfig-cli.exe` in the
   install folder).
2. **Optionally install a TeX** (TeX Live, MacTeX, MiKTeX) for LaTeX labels:
   RustFig finds `pdflatex` or `lualatex` on its own.
3. **Add the server to the client:**
   - **Claude Code:** `claude mcp add --scope user rustfig -- rustfig-cli mcp`
     (`--scope user` for all projects; leave it out for the current one).
     `claude mcp list` shows it; `/mcp` in a session shows its state.
   - **OpenAI Codex CLI:** `codex mcp add rustfig -- rustfig-cli mcp`, or in
     `~/.codex/config.toml`:
     ```toml
     [mcp_servers.rustfig]
     command = "rustfig-cli"
     args = ["mcp"]
     ```
   - **Claude Desktop and other clients** with a JSON configuration
     (`claude_desktop_config.json`, Cursor's `mcp.json`, …):
     ```json
     {"mcpServers": {"rustfig": {"command": "/opt/homebrew/bin/rustfig-cli", "args": ["mcp"]}}}
     ```
4. **Install the skill** (recommended): [`skills/rustfig/SKILL.md`](../skills/rustfig/SKILL.md)
   tells the assistant how to use the tools: the workflow, coordinates,
   LaTeX texts, plots, exports and the usual mistakes. Copy the folder
   `skills/rustfig` to `~/.claude/skills/rustfig` (Claude Code, all
   projects), to `.claude/skills/rustfig` in a project, or to
   `~/.codex/skills/rustfig` (Codex). Clients without skills can be given
   the file's text as instructions.
5. **Restart the client** after updating RustFig: a running server keeps
   the old program.

Try it: "Draw a blue rounded box with a red circle in the middle and export
it as PDF." More examples, with the figures they gave, are in the manual's
[AI assistants](manual/ai-assistants.md) topic and in [`examples/ai`](../examples/ai).

**Online assistants** (claude.ai, ChatGPT in the browser) can't use RustFig
yet: they reach only remote MCP servers over HTTPS, and `rustfig-cli mcp`
runs on the user's computer over stdio.

## Tools

| Tool | Purpose |
|---|---|
| `rustfig_commands` | list commands and their parameters (`filter`: a substring of the id or label; the full list is about 50 KB) |
| `rustfig_execute` | run a command: `{"command": "shape.circle", "params": {"cx": 600, "cy": 600, "r": 300}}` |
| `rustfig_render` | the drawing as a PNG image (`dpi`, default 100; `latex`, default true: LaTeX texts typeset by TeX when one is installed, else shown as their source, with the page fitted to the source; TeX's errors, such as an undefined macro, come back as warnings next to the image) |
| `rustfig_document` | the drawing as Fig 3.2 text (or `format: "rustfig"`) |

Coordinates are Fig units: 1200 per inch, y grows downwards. Open and save
files with `file.open` / `file.save` through `rustfig_execute`.

## Plots

`plot.add` draws a plot as one compound and `plot.update` redraws it with
changed parameters (`plot.get` returns them; `rustfig_commands` with the
filter `plot` lists every parameter). Everything the Plot dialog offers is a
parameter, and a plot's parameters are kept in the figure, so an edited
plot redraws the same. Axes and grid (the plot's `axes`, `axis_arrows`,
`grid`, `grid_minor` and `grid_style`):

- `axes`: `"frame"` (ticks and numbers on the edges of the box, the default),
  `"origin"` (axes crossing at (0, 0) or the nearest edge, arrowheads at the
  tips, labels at the tips, ticks and numbers on one side) or `"none"`.
  `axis_arrows: false` leaves the arrowheads off.
- `grid: true` with `grid_minor: true` (lines at the minor ticks too) and
  `grid_style`: `"dotted"` (default), `"dashed"`, `"solid"`.

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First version. |
| 2026-11-06 | Setup for Claude Code, Codex and JSON-configured clients; the RustFig skill (`skills/rustfig/SKILL.md`). |
| 2026-11-11 | Plots: axes crossing at the origin, no axes, grid with minor lines and a line style (Q1). |
