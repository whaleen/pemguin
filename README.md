# pemguin 🐧

Terminal project manager for developers who live in the CLI.

## What it does

`pm` / `pemguin` is a Ratatui TUI that keeps your dev projects organized. For each project it surfaces config status, GitHub issues, prompts, memory files, skills, MCP servers, and project tools — all without leaving the terminal.

Startup scanning and GitHub-backed project data load in the background, so the UI stays responsive while data hydrates.

## Install

```bash
git clone https://github.com/whaleen/pemguin
cd pemguin/cli
cargo install --path .
```

This installs both `pm` and `pemguin`.

Requires: Rust stable, `gh` CLI (authenticated), Nerd Font terminal.

## Usage

```bash
pm
pemguin
```

Navigate with `↑↓` or `jk`. Press `enter` to open a project. `esc` goes back. `q` quits.

### Project tabs

| Key | Tab |
|-----|-----|
| `1` | Home — repo info, description, recent commits |
| `2` | Issues — open GitHub issues, copy prompt to work on one |
| `3` | Config — managed repo files like AGENT.md, SPEC.md, .gitignore, prompts, memory |
| `4` | Prompts — global (`~/.pemguin/prompts/`) and project (`.prompts/`) |
| `5` | Memories — `.memory/`, `~/.pemguin/memory/`, `.claude/.../memory/` |
| `6` | Skills — installed skills from `skills-lock.json` |
| `7` | MCP — configured servers from `.mcp.json` |
| `8` | Pane — launch project tools like `lazygit`, `yazi`, and `$EDITOR` |

### Projects root

| Key | Action |
|-----|--------|
| `r` | Refresh selected project row |
| `R` | Rescan all projects |
| `s` | Sync GitHub metadata |
| `enter` | Open project |
| `q` | Quit |

## Configuration

`~/.pemguin.toml`:

```toml
[projects]
root = "~/Projects"   # directory to scan (2 levels deep for .git)

[theme]
accent  = "#e8b887"   # hot-reloaded on file change — no restart needed
sel_fg  = "#101010"
fg_dim  = "#A0A0A0"
fg_xdim = "#7E7E7E"
border  = "#232323"
surface = "#1C1C1C"
green   = "#90b99f"
red     = "#f5a191"
yellow  = "#e6b99d"
purple  = "#aca1cf"
```

Or set `PEMGUIN_PROJECTS_DIR` env var for projects root.

Theme changes are detected within ~50ms. If using the whaleen dotfiles, run `theme/generate.sh` to propagate palette changes to all tools including pemguin.

## Project structure

```
pemguin/
  cli/            — Rust TUI source
  prompts/        — built-in prompt templates
  stacks/         — stack reference sheets
  docs/           — architecture and feature docs
  AGENT.md        — agent context for this repo
  SPEC.md         — feature checklist
  CONSTITUTION.md — universal dev principles
```
