# pemguin — Architecture Overview

## Design Philosophy

Observer-first, modifier-second, zero-config. pemguin reads from where agents naturally store things. It does not reproduce agent data in project directories or steer agents toward any structure. The only config it writes is `~/.pemguin.toml` (projects root + theme).

## Structure

Everything lives in `cli/src/lib.rs` (~9000 lines). Two entry surfaces:

- **TUI**: Ratatui application started by `pemguin::start()`
- **CLI**: machine-oriented commands dispatched by `pemguin::run_cli()`

`src/main.rs` and `src/bin/pm.rs` both route to CLI when subcommands are present, otherwise start the TUI. Backend: Ratatui + Crossterm.

## Screen Model

```
Screen::Projects          — root project list
Screen::InProject(tab)    — drilled into a project, one of 7 tabs
```

Tabs: `Home | Issues | Config | Prompts | Memories | Agents | Pane`

The Agents tab has three sub-sections navigated with `[`/`]`: `Mcp | Skills | Sessions`

## Application State (`App`)

Key fields:

- `screen: Screen` — current screen + active tab
- `projects: Vec<Project>` — scanned project list
- `project_entries: Vec<ProjectEntry>` — flat render list (Group headers + Item indices)
- `repo: String` — active project's `owner/repo` slug
- Per-tab state: `issue_list_state`, `setup_items`, `memory_files`, `skills`, `mcp_servers`, `sessions`, pane state, etc.

## Layout

Every InProject screen uses a 2-row header:

```
┌──────────────────────────────────────────┐
│  header row: 🐧 pm  repo-name  branch    │
│  nav row:  1 home  2 issues  3 config …  │
├──────────────────────────────────────────┤
│  content area                            │
├──────────────────────────────────────────┤
│  footer: key hints                       │
└──────────────────────────────────────────┘
```

Some tabs add variable-height rows for inputs and status messages.

## Key Handling

`handle_key()` dispatches in layers:

1. `Ctrl+C` — always quit
2. Global InProject handlers: `Esc` → back, `q` → quit, `Tab`/number keys → switch tab
3. Tab-specific handler: `handle_home`, `handle_issues`, `handle_setup`, `handle_memories`, etc.

Sub-flows (prompt fill, home edit, memory input) capture all keys and suppress global nav until dismissed.

## Project Scanning

`scan_projects()` walks at most 2 levels from the configured root:

```
~/Projects/
  repo-a/         ← level 1, .git present → included, group=""
  _org/           ← level 1, no .git
    repo-b/       ← level 2, .git present → included, group="_org"
    repo-c/cli/   ← level 3, NOT found
```

`project_info()` runs git inspection and setup checks per directory. Scanning is parallelized across worker threads. Initial scan runs in the background after TUI opens.

## Agent Storage Readers

Native agent storage is read directly — no intermediate files. See `docs/agents/` for the full storage interface spec for each agent.

### Sessions (`resolve_sessions`)

Called when the Sessions sub-section is first opened. Reads from:

- **Claude**: `claude_project_dirs()` returns matching `~/.claude/projects/<encoded>/` dirs (checks both v1 and v2 path encoding). Scans JSONL files.
- **Codex**: `import_codex_sessions()` walks `~/.codex/sessions/YYYY/MM/DD/` and matches `cwd` field in the `session_meta` first line.
- **Gemini**: `import_gemini_sessions()` reads `~/.gemini/projects.json` for the project name, then scans `~/.gemini/tmp/<name>/chats/` JSON files.
- **Pi**: `import_pi_sessions()` uses `pi_encode_path()` to find `~/.pi/agent/sessions/<encoded>/` and reads JSONL files.

### Memories (`reload_memories`)

Three views switchable within the Memories tab:

- **Claude** (`c`): `~/.claude/projects/<encoded>/memory/*.md`
- **Codex** (`x`): `~/.codex/memories/<repo-name>/*.md`
- **Gemini** (`g`): `~/.gemini/GEMINI.md` (single file, global)

### Path Encoding

| Agent | Rule | Example |
|-------|------|---------|
| Claude | non-alphanumeric → `-` (two variants: `_` preserved or converted) | `-Users-josh-Projects--foo` |
| Codex | date-bucketed; match by `cwd` field | n/a |
| Gemini | project name from `~/.gemini/projects.json` | `astrds` |
| Pi | strip leading `/`, replace `/` with `-`, wrap in `--` | `--Users-josh-Projects-_foo--` |

## Data Flow

```
App::new()
  → spawn background scan_projects()
  → render root immediately (shows "Scanning…")

On project open (switch_project):
  → start_home_load() — spawns background thread for gh API calls
  → lazy tab loading via ensure_tab_loaded()

On tab open:
  → Memories: reload_memories() reads native agent storage
  → Agents/Sessions: resolve_sessions() scans all 4 agent stores
  → Agents/Skills: load_skills() reads ~/.agents/.skill-lock.json
  → Agents/MCP: load_mcp_servers() reads .mcp.json + ~/.claude.json
```

## Agent Storage Maintenance

When an agent updates its storage format:

1. Validate against disk using the checklist in `docs/agents/<agent>.md`
2. Update `docs/agents/<agent>.md`
3. Update the relevant reader function in `cli/src/lib.rs`

Reader functions by agent:
- Claude: `claude_project_dirs()`, `resolve_sessions()`, `claude_memory_path()`
- Codex: `import_codex_sessions()`, `parse_codex_session()`, `codex_memory_dirs()`
- Gemini: `import_gemini_sessions()`, `gemini_memory_path()`
- Pi: `import_pi_sessions()`, `pi_encode_path()`
