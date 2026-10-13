# MCP server

**Status:** 2026-10-13.

`rustfig-cli mcp` speaks the Model Context Protocol (JSON-RPC 2.0, one
message per line) on stdin/stdout. Example client configuration:

```json
{"mcpServers": {"rustfig": {"command": "rustfig-cli", "args": ["mcp"]}}}
```

## Tools

| Tool | Purpose |
|---|---|
| `rustfig_commands` | list commands and their parameters (optional `filter`) |
| `rustfig_execute` | run a command: `{"command": "shape.circle", "params": {"cx": 600, "cy": 600, "r": 300}}` |
| `rustfig_render` | the drawing as a PNG image (`dpi`, default 100) |
| `rustfig_document` | the drawing as Fig 3.2 text (or `format: "rustfig"`) |

Coordinates are Fig units: 1200 per inch, y grows downwards. Open and save
files with `file.open` / `file.save` through `rustfig_execute`.

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First version. |
