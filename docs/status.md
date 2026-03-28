# pemguin — Status

## What's Working

Everything in SPEC.md marked ✅ is shipped. The core loop works end to end:

- `pm` opens, scans projects, groups by org directory
- Startup project scanning happens in the background; the TUI renders immediately
- Opening a project is immediate; Home and Issues hydrate in the background, other tabs load lazily on first visit
- All 8 tabs navigate correctly; Esc returns to the project list
- Prompts fill placeholders and copy to clipboard
- Memory files can be created, edited, deleted, and migrated
- Setup tab detects and applies missing AGENT.md, SPEC.md, .mcp.json, skills-lock.json

## Known Rough Edges

- **`gh` dependency**: most features silently degrade without a working `gh` auth. No clear error state shown in the UI when `gh` is unavailable.
- **Nerd Font hard requirement**: no graceful fallback for terminals without Nerd Font support.
- **Pane tab is a placeholder**: tab 8 renders ASCII art only — no PTY, no child process.
- **Single source file**: `cli/src/main.rs` is ~2500 lines. Navigation is fine but it will become hard to maintain as features are added.
- **Project selection on rescan resets**: a full background rescan rebuilds the list state and does not yet preserve the previous selection.

## What's Next

- Embedded child pane via `tui-term` — Yazi first, then Helix
- Project search / filter on the projects list
- Preserve selection and scroll position across project rescans
- Create and edit prompts from within pm
- Install / remove skills and MCP servers from within pm
