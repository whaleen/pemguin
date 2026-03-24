---
name: project
description: pemguin (pm) project overview — terminal project manager TUI in Rust
type: project
---

**pemguin** (`pm` CLI) is a terminal project manager TUI for Joshua's development workflow.

## Stack
- Rust + Ratatui (crossterm backend)
- Main source: `cli/src/main.rs`

## Features
- Projects list with org grouping and nerd font icons
- Per-project tabs: Home, Issues, Setup, Prompts, Memories
- Home tab: chafa avatar, GitHub description/homepage editing (`gh repo edit`), URL copy
- Issues tab: GitHub issues via `gh` CLI
- Setup tab: project setup checklist
- Prompts tab: `.prompts/` file browser
- Memories tab: `.memory/` file browser with create/edit/delete/migrate, `$EDITOR` integration

## Nerd Font Icons
Icons are double-width (2 terminal cells). Must compensate padding in format strings. Use explicit positional args in format strings — e.g. `format!("{} {:<10}", I_BRANCH, "branch")`.

## Key Patterns
- `pending_editor: Option<PathBuf>` — checked in main run() loop to suspend/resume TUI around $EDITOR
- Tab cycling: Home→Issues→Setup→Prompts→Memories→Home; key `5` = Memories
- `switch_project()` loads memories (defaults to Claude view if has content) and avatar

## Memory System
- Per-project: `.memory/MEMORY.md` + individual `.memory/*.md` files
- Global: `~/.pemguin/memory/MEMORY.md` + individual files
- Claude path formula: replace all non-alphanumeric chars with `-`
  - e.g. `/Users/josh/Projects/_whaleen/tiles` → `-Users-josh-Projects--whaleen-tiles`
  - Claude memory: `~/.claude/projects/-Users-josh-Projects--whaleen-tiles/memory/`
