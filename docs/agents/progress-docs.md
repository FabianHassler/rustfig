# Roadmap and progress docs

What to update when a task lands (linked from `CLAUDE.md`). The progress docs follow craftrules' [`standards/progress-docs.md`](https://github.com/storytold/craftrules/blob/main/standards/progress-docs.md):

- [`ROADMAP.md`](../../ROADMAP.md): stage, headline numbers, dimensions, features, languages, upcoming, progress log.
- [`docs/plan.md`](../plan.md): phased tasks with ids, acceptance criteria and hours; the user's decisions.
- [`docs/target-app-parity.md`](../target-app-parity.md): the authoritative parity assessment (area table, methodology, calibration, evidence of what ships).
- [`docs/source-map.md`](../source-map.md): every xfig/fig2dev C file → RustFig module, status, task.
- [`docs/gaps.md`](../gaps.md): every known shortfall, ranked. [`docs/roadmap.md`](../roadmap.md): milestones and current focus.
- Checklists: [`ui-parity.md`](../ui-parity.md), [`keys-parity.md`](../keys-parity.md), [`file-format-parity.md`](../file-format-parity.md), [`hardware-parity.md`](../hardware-parity.md), [`localization-parity.md`](../localization-parity.md), [`effects-parity.md`](../effects-parity.md), [`type-parity.md`](../type-parity.md); [`architecture.md`](../architecture.md), [`development.md`](../development.md), [`mcp.md`](../mcp.md).

When a task lands, update in the same commit: the manual topic that explains the feature to users (every user-visible feature is explained in the help), the task row in `docs/plan.md`, the area row in `docs/target-app-parity.md` (score, missing items, hours) and its evidence list, the gap entry in `docs/gaps.md` if it closed or shrank, the checklist it belongs to, the milestone row in `docs/roadmap.md`, and the ROADMAP's tables and progress log when a headline number moves (commands, tests, formats, perf, parity). Update each touched doc's status line and revision history.

- Keep the README's Status section in step with the ROADMAP headline.
