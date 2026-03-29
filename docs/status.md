# pemguin — Status

## What's Working

Everything in SPEC.md marked ✅ is shipped. The core loop works end to end:

- `pm` opens, scans projects, groups by org directory
- Startup project scanning happens in the background; the TUI renders immediately
- Opening a project is immediate; Home and Issues hydrate in the background, other tabs load lazily on first visit
- All 9 tabs navigate correctly; Esc returns to the project list
- Config tab safely applies missing managed files, edits them in `$EDITOR`, deletes them, and resets them to pemguin defaults
- Prompts fill placeholders and copy to clipboard
- Memory files can be created, edited, deleted, and migrated
- Pane tab launches project tools like `lazygit`, `yazi`, and `$EDITOR`
- Sessions tab (tab 9) lists Claude Code and Codex sessions per project, with inline summary, export to `.pemguin/exports/`, and copy-to-clipboard launch/resume commands

## Known Rough Edges

- **`gh` dependency**: most features silently degrade without a working `gh` auth. No clear error state shown in the UI when `gh` is unavailable.
- **Nerd Font hard requirement**: no graceful fallback for terminals without Nerd Font support.
- **Single source file**: `cli/src/main.rs` is ~5500+ lines. Navigation is fine but it will become hard to maintain as features are added.
- **Codex session scan performance**: the Sessions tab does a full walk of `~/.codex/sessions/YYYY/MM/DD/` on every open. For large Codex histories this can be slow. Future fix: cache last-scanned state and do incremental updates.
- **Project selection on rescan resets**: a full background rescan rebuilds the list state and does not yet preserve the previous selection.
- **Config reset is blunt**: reset rewrites pemguin-managed sample content, but does not yet show a confirmation step.

## What's Next

- Embedded child pane via `tui-term` — Yazi first, then Helix
- Project search / filter on the projects list
- Preserve selection and scroll position across project rescans
- Create and edit prompts from within pm
- CRUD for issues, skills, MCP servers, and other repo metadata from within pm
- Install / remove skills and MCP servers from within pm
