# AI assistants

RustFig can be driven by an AI assistant: you describe a figure in words, and the assistant draws it with RustFig's commands, looks at the result, fixes it, saves the .fig file and exports a PDF. The figure is an ordinary xfig drawing, which you open in RustFig (or xfig) afterwards and edit by hand.

This works with assistants that run on your computer and can start programs: **Claude Code**, **Claude Desktop**, the **OpenAI Codex CLI** and **Cursor**, among others. They talk to `rustfig-cli mcp`, RustFig's server for the Model Context Protocol (MCP).

## What it looks like

Three figures made this way, each from one sentence. The pictures are on the [website](https://fabianhassler.github.io/rustfig/#ai); the files (the drawing, its PDF and the tool calls the assistant made) are in the repository's [`examples/ai`](https://github.com/FabianHassler/rustfig/tree/main/examples/ai) folder.

- **"Draw the block diagram of a feedback loop: controller C(s), plant G(s) and a sensor H(s) in the feedback path, a summing junction with + and −, the signals r, e, u, y labeled in LaTeX. Save it as feedback-loop.fig and export a PDF."** → three boxes with typeset labels, the summing junction, arrows for the forward and the feedback path, and the files `feedback-loop.fig` and `feedback-loop.pdf`.
- **"Plot the double-well potential V(x) = V0 (x²/a² − 1)² for x/a from −1.6 to 1.6 with LaTeX axis labels, mark the two minima and the barrier height ΔV = V0."** → a plot with LaTeX axis labels and tick numbers, dots at the minima, a dashed line between them and a double arrow labeled ΔV = V₀.
- **"Draw the energy-level diagram of a three-level Λ system: ground states |1⟩ and |2⟩, excited state |3⟩, the probe Ω_p (blue) from |1⟩ and the coupling Ω_c (red) from |2⟩, labels in LaTeX."** → three levels with ket labels and the two colored transitions.

More things to ask:

- "Open setup.fig, make all labels LaTeX and export it as PDF with the labels typeset."
- "Move the legend of plot.fig to the top right corner and make all lines twice as thick."
- "Draw a Mach–Zehnder interferometer: laser, two beam splitters, two mirrors, two detectors."
- "Export every .fig file in this folder as PDF and tell me which ones have warnings."

An assistant can make mistakes, as a person can: look at the figure before you use it. It sees the figure itself (it renders it as a picture and reads the warnings), so asking it to fix something usually works ("the label overlaps the curve, move it to the right").

## Setting it up

1. **Install RustFig** (Homebrew, the Linux packages, Scoop or winget, or a release archive; see [Getting started](getting-started.md)). Check that `rustfig-cli --version` works in a terminal.
2. **Optionally install TeX** (TeX Live, MacTeX or MiKTeX) for LaTeX labels. RustFig finds `pdflatex` or `lualatex` by itself.
3. **Add RustFig to the assistant:**
   - **Claude Code:** run `claude mcp add --scope user rustfig -- rustfig-cli mcp` in a terminal. `claude mcp list` shows it, `/mcp` inside a session shows its state.
   - **OpenAI Codex CLI:** run `codex mcp add rustfig -- rustfig-cli mcp`, or add to `~/.codex/config.toml`:

     ```toml
     [mcp_servers.rustfig]
     command = "rustfig-cli"
     args = ["mcp"]
     ```

   - **Claude Desktop:** Settings > Developer > Edit Config, and add RustFig to `claude_desktop_config.json`:

     ```json
     {"mcpServers": {"rustfig": {"command": "/opt/homebrew/bin/rustfig-cli", "args": ["mcp"]}}}
     ```

     Desktop apps don't always see the terminal's search path, so give the full path: `which rustfig-cli` prints it (on Windows, `where rustfig-cli`).
   - **Cursor:** the same JSON in `~/.cursor/mcp.json` (all projects) or `.cursor/mcp.json` in a project.
4. **Install the RustFig skill** (recommended). The skill is a page of instructions that tells the assistant how to use RustFig well: coordinates, LaTeX labels, plots, exports and the usual mistakes. Copy the folder [`skills/rustfig`](https://github.com/FabianHassler/rustfig/tree/main/skills/rustfig) from the repository to `~/.claude/skills/rustfig` (Claude Code, all projects), to `.claude/skills/rustfig` in a project, or to `~/.codex/skills/rustfig` (Codex). Assistants without skills can be given the file's text as instructions.
5. **Restart the assistant** (or its MCP connection) after updating RustFig: a running server keeps the old program.

Then ask for a figure. Most assistants ask for your permission before they use RustFig's tools the first time.

## How it works

The assistant has four tools: one lists RustFig's commands, one runs a command, one renders the drawing as a picture for the assistant to look at, and one returns the drawing as Fig text. The commands are the same ones the menus and modes run ([Automation](automation.md)), so whatever you can do in the window an assistant can do too, and every change is one undo step.

Parameters follow the window. A pen or fill color is a color number, `#rrggbb`, `#rgb` or an X11 name as in the color dialog: a color the figure has is reused, any other is added as a user color in the same undo step (an error says when all 512 are used). Arrow types can be named (`dot`, `circle`, `bar`, `diamond`, ...) as well as numbered 0 to 14. A text with line breaks becomes one text object per line, spaced as Return does in Text mode. The picture the assistant looks at can be cut to the drawing's bounds instead of the page, as View > Bounding box > Fit to drawing does, without changing the figure's own box. For braces, block arrows, zigzags, springs and coils the assistant searches the bundled libraries (`library.search`, `library.place`) instead of drawing them.

The server holds one drawing at a time, in memory, and writes files only when the assistant saves or exports. It doesn't open a window; open the saved .fig in RustFig to see or edit it.

## Online assistants

The web versions of Claude (claude.ai) and ChatGPT can't use RustFig yet: they reach only servers on the internet, and RustFig's server runs on your computer. Use the desktop app or one of the command-line assistants above. The web version of RustFig ([try it in your browser](https://fabianhassler.github.io/rustfig/app/)) has no assistant connection either.
