# pemguin — Status

## What's Working

Everything in SPEC.md marked ✅ is shipped. The core loop works end to end:

- `pm` opens, scans projects, groups by org directory
- Opening a project loads GitHub data, issues, setup checklist, prompts, memories, skills, MCP servers
- All 8 tabs navigate correctly; Esc returns to the project list
- Prompts fill placeholders and copy to clipboard
- Memory files can be created, edited, deleted, and migrated
- Setup tab detects and applies missing AGENT.md, SPEC.md, .mcp.json, skills-lock.json

## Known Rough Edges

- **Synchronous I/O**: project open blocks the UI while `gh` CLI calls complete. No loading indicator.
- **`gh` dependency**: most features silently degrade without a working `gh` auth. No clear error state shown in the UI when `gh` is unavailable.
- **Nerd Font hard requirement**: no graceful fallback for terminals without Nerd Font support.
- **Pane tab is a placeholder**: tab 8 renders ASCII art only — no PTY, no child process.
- **Single source file**: `cli/src/main.rs` is ~2500 lines. Navigation is fine but it will become hard to maintain as features are added.

## What's Next

- Async data loading with a loading indicator (removes the UI freeze on project open)
- Embedded child pane via `tui-term` — Yazi first, then Helix
- Project search / filter on the projects list
- Create and edit prompts from within pm
- Install / remove skills and MCP servers from within pm
