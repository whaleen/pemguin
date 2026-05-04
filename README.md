# pemguin 🐧

Terminal project manager for developers who work with AI agents.

`pm` / `pemguin` is a Ratatui TUI that gives you a single place to navigate all your local git repos. It surfaces GitHub issues, context file status, agent memories, skills, MCP servers, and session history — without leaving the terminal.

**Observer-first.** pemguin reads from where agents naturally store things. It does not reproduce agent data in its own directories or steer agents toward any structure.

## Install

```bash
git clone https://github.com/whaleen/pemguin
cd pemguin/cli
cargo install --path .
```

Installs both `pm` and `pemguin`.

Requires: Rust stable, `gh` CLI (authenticated), Nerd Font terminal.

## Usage

```bash
pm
pemguin
```

Navigate with `↑↓` or `jk`. Press `enter` to open a project. `esc` goes back. `q` quits.

### Project tabs

| Key | Tab | What it shows |
|-----|-----|---------------|
| `1` | Home | Repo info, description, recent commits, stack |
| `2` | Issues | Open GitHub issues; copy prompt to work on one |
| `3` | Config | Native agent context files: CLAUDE.md, AGENTS.md, GEMINI.md, .mcp.json |
| `4` | Prompts | Prompt templates (global and project-level) |
| `5` | Memories | Native agent auto-memory: Claude (`c`), Codex (`x`), Gemini (`g`) |
| `6` | Agents | MCP servers, installed skills, and per-project sessions |
| `7` | Pane | Launch project tools: lazygit, yazi, $EDITOR |

### Sessions (Agents tab)

Sessions are discovered directly from each agent's native storage:

| Agent | Storage location |
|-------|-----------------|
| Claude Code | `~/.claude/projects/<encoded>/` JSONL |
| Codex | `~/.codex/sessions/YYYY/MM/DD/` JSONL |
| Gemini CLI | `~/.gemini/tmp/<project-name>/chats/` JSON |
| Pi | `~/.pi/agent/sessions/<encoded>/` JSONL |

`n` — copy a new session launch command to clipboard  
`y`/Enter — copy a resume command for the selected session  
`s` — inline session summary (Claude + Pi)

### Memories (Memories tab)

| Key | View | Source |
|-----|------|--------|
| `c` | Claude | `~/.claude/projects/<encoded>/memory/` |
| `x` | Codex | `~/.codex/memories/<repo-name>/` |
| `g` | Gemini | `~/.gemini/GEMINI.md` (global) |

`e`/Enter edits the selected file. `n` creates a new memory file (or opens GEMINI.md for the Gemini view). `d` deletes.

### MCP server

`pm` can run as a local stdio MCP server:

```bash
pm mcp serve
```

Exposed tools: `pemguin_project_inspect`, `pemguin_setup_plan`, `pemguin_agent_instructions`

Example `.mcp.json` entry:

```json
{
  "mcpServers": {
    "pemguin": {
      "command": "pm",
      "args": ["mcp", "serve"]
    }
  }
}
```

## Configuration

`~/.pemguin.toml`:

```toml
[projects]
root = "~/Projects"   # scanned 2 levels deep for .git dirs

[theme]
accent  = "#e8b887"   # hot-reloaded on file change
sel_fg  = "#101010"
fg_dim  = "#A0A0A0"
fg_xdim = "#7E7E7E"
green   = "#90b99f"
red     = "#f5a191"
yellow  = "#e6b99d"
purple  = "#aca1cf"
```

Set `PEMGUIN_PROJECTS_DIR` to override `projects.root` via env.

Theme changes are detected within ~50ms — no restart needed.

## Supported agents

pemguin reads native storage for four agents:

| Agent | Binary | Sessions | Memory | Skills |
|-------|--------|----------|--------|--------|
| Claude Code | `claude` | `~/.claude/projects/` | `~/.claude/projects/<enc>/memory/` | `~/.claude/skills/` |
| Codex | `codex` | `~/.codex/sessions/` | `~/.codex/memories/` | `~/.codex/skills/` |
| Gemini CLI | `gemini` | `~/.gemini/tmp/` | `~/.gemini/GEMINI.md` | `~/.gemini/skills/` |
| Pi | `pi` | `~/.pi/agent/sessions/` | — | `~/.pi/agent/skills/` |

See `docs/agents/` for the detailed storage interface docs used to implement each reader.

## Project structure

```
pemguin/
  cli/          — Rust TUI + CLI source
  docs/
    agents/     — storage interface docs for each supported agent
    architecture/ — app structure and data flow
  templates/    — built-in prompt and doc templates
```
