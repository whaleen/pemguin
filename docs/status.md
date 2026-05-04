# pemguin — Status

## What's Working

- `pm` opens, scans projects, groups by org directory
- Background project scanning; TUI renders immediately, data hydrates as it arrives
- Opening a project is immediate; Home and Issues hydrate in the background, other tabs load lazily
- 7 tabs navigate correctly; Esc returns to the project list
- **Config tab** shows native agent context files (CLAUDE.md, AGENTS.md, GEMINI.md, .mcp.json) as informational status — no pemguin-managed files or apply actions
- **Prompts tab** fills placeholders, copies to clipboard
- **Memories tab** reads directly from native agent storage:
  - Claude: `~/.claude/projects/<encoded>/memory/`
  - Codex: `~/.codex/memories/<repo-name>/`
  - Gemini: `~/.gemini/GEMINI.md`
  - View+edit; new files can be created in Claude/Codex memory dirs
- **Agents tab** consolidates MCP servers, installed skills, and per-project sessions
  - Sessions discovered from native storage for all 4 agents (Claude, Codex, Gemini, Pi)
  - Inline session summary and resume command copy for Claude and Pi sessions
- **Pane tab** launches project tools (lazygit, yazi, $EDITOR)
- MCP server: `pm mcp serve` — exposes `pemguin_project_inspect`, `pemguin_setup_plan`, `pemguin_agent_instructions`

## Known Rough Edges

- **`gh` dependency**: most Home and Issues features silently degrade without working `gh` auth. No clear error state in the UI.
- **Nerd Font hard requirement**: no graceful fallback for terminals without Nerd Font support.
- **Single source file**: `cli/src/lib.rs` is ~9000 lines. Functional but will become hard to maintain.
- **Codex session scan performance**: full walk of `~/.codex/sessions/YYYY/MM/DD/` on every project open. Can be slow for large Codex histories.
- **Gemini session summary**: `s` (inline summary) does not yet work for Gemini sessions (they are JSON, not JSONL; parser not yet implemented).
- **Gemini legacy sessions**: sessions in SHA-256 named dirs (older Gemini versions) are not yet scanned — only the human-readable name dir from `projects.json` is checked.
- **Project selection on rescan resets**: full background rescan rebuilds list state and does not preserve previous selection.

## What's Next

- Gemini session summary viewer (parse JSON session format)
- Gemini legacy session dir scanning (SHA-256 named dirs)
- Embedded child pane via `tui-term` — Yazi first
- Project search / filter on the projects list
- Preserve selection and scroll position across rescans
- CRUD for issues and repo metadata from within pm
