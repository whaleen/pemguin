# pemguin — Agent Context

## What This Project Is

pemguin (`pm` / `pemguin`) is a terminal project manager TUI built with Ratatui. It gives developers a single place to navigate all their local git repos — viewing GitHub issues, checking project setup status, browsing prompts and memory files, and inspecting installed skills and MCP servers. It is agent-agnostic: designed for any AI-assisted dev workflow, not tied to a specific tool.

## Stack

- **Language**: Rust (stable)
- **TUI**: Ratatui 0.29 + Crossterm 0.28
- **External tools**: `gh` CLI (GitHub operations), `chafa` (avatar rendering), `$EDITOR` (file editing)
- **Config**: `~/.pemguin.toml` (TOML), `~/.pemguin/` runtime dir (prompts, memory, avatars, cache)
- **Deployment**: local binary via `cargo install --path cli`

## Running Locally

```bash
cd cli
cargo run                 # dev build
cargo install --path .    # install to ~/.cargo/bin/{pm,pemguin}
```

Prerequisites: Rust stable, `gh` CLI authenticated, Nerd Font terminal. `chafa` is optional (org avatar rendering).

Config: `~/.pemguin.toml` — set `projects.root` to your projects directory. Defaults to `~/Projects`.

## Key Files & Directories

```
cli/
  src/main.rs       — entire application (single file)
  Cargo.toml        — dependencies
prompts/            — built-in prompt templates (work-on-issue, deploy)
stacks/             — stack reference sheets (rust, vite-react, etc.)
docs/               — architecture and feature documentation
CONSTITUTION.md     — universal dev principles, referenced by agents globally
~/.pemguin.toml     — runtime config (projects root dir)
~/.pemguin/
  prompts/          — global prompts shown in every project's Prompts tab
  memory/           — global memory files
  avatars/          — cached GitHub org avatar images (chafa ANSI art)
  cache.json        — GitHub metadata cache
```

## Gotchas

- **Single source file**: all TUI state, rendering, key handling, and data loading live in `cli/src/main.rs`.
- **2-level scan**: `scan_projects()` walks at most 2 levels deep from the configured root. The `.git` dir must be at level 1 or 2 — repos nested deeper won't appear.
- **`gh` CLI required**: issues, descriptions, homepage edits, and GitHub sync all shell out to `gh`. The app degrades gracefully when unavailable but most features won't work.
- **Nerd Font required**: tab icons and status indicators use Nerd Font codepoints. Without a Nerd Font the UI shows replacement characters.
- **Pane tab (8) is a placeholder**: reserved for an embedded child TUI (Yazi, Helix) via `tui-term`. PTY plumbing is not implemented yet; `Ctrl+W` is reserved for pane focus handoff.

## Spec

See SPEC.md for the feature checklist.

## Docs

- `docs/architecture/overview.md` — app structure, state, key handling, data flow, scan logic
- `docs/status.md` — what's working, known rough edges, what's next
