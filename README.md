# pemguin 🐧

Terminal project manager for developers who live in the CLI.

## What it does

`pm` is a Ratatui TUI that keeps your dev projects organized. For each project it surfaces config status, GitHub issues, prompts, memory files, skills, MCP servers, and project tools — all without leaving the terminal.

Startup scanning and GitHub-backed project data load in the background, so the UI stays responsive while data hydrates.

## Install

```bash
git clone https://github.com/whaleen/pemguin
cd pemguin/cli
cargo install --path .
```

Requires: Rust stable, `gh` CLI (authenticated), Nerd Font terminal.

## Usage

```bash
pm
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
```

Or set `PEMGUIN_PROJECTS_DIR` env var.

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
