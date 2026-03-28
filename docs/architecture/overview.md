# pemguin — Architecture Overview

## Structure

pemguin is a single-file Rust TUI application. All state, rendering, key handling, data loading, and GitHub integration live in `cli/src/main.rs`. The TUI is built on Ratatui + Crossterm.

## Screen Model

Two top-level screens:

```
Screen::Projects          — root project list
Screen::InProject(tab)    — drilled into a project, showing one of 8 tabs
```

Tab variants: `Home | Issues | Setup | Prompts | Memories | Skills | Mcp | Pane`

## Application State (`App`)

Key fields:

- `screen: Screen` — current screen + active tab
- `projects: Vec<Project>` — scanned project list
- `project_entries: Vec<ProjectEntry>` — flat render list (Group headers + Item indices)
- `repo: String` — active project's `owner/repo` slug
- `context: String` — `"owner/repo (branch)"` used for prompt auto-fill
- Per-tab state: `issue_list_state`, `setup_items`, `prompt_state`, `memory_files`, `skills`, `mcp_servers`, etc.

## Layout

Every InProject screen uses a 2-row header:

```
┌──────────────────────────────────────────┐
│  header row: 🐧 pm  repo-name  branch    │  ← identity
│  nav row:  1 home  2 issues  3 setup …   │  ← tabs
├──────────────────────────────────────────┤
│  content area (Min(0))                   │
├──────────────────────────────────────────┤
│  footer: key hints                       │
└──────────────────────────────────────────┘
```

Some tabs add a variable-height middle row (edit input, status message).

## Key Handling

`handle_key()` dispatches in layers:

1. `Ctrl+C` — always quit
2. Global InProject handlers (when not in a sub-flow): `Esc` → back, `q` → quit, `Tab` / number keys → switch tab
3. Sub-screen handler: `handle_home`, `handle_issues`, `handle_prompts`, etc.

Sub-flows (prompt fill, home edit, memory input) capture all keys and suppress global nav until dismissed with `Esc` or `Enter`.

## Project Scanning

`scan_projects()` walks at most 2 levels from the configured root:

```
~/Projects/
  repo-a/           ← level 1, .git present → included, group=""
  _org/             ← level 1, no .git
    repo-b/         ← level 2, .git present → included, group="_org"
    repo-c/cli/     ← level 3, .git present → NOT found
```

For each found directory, `project_info()` runs git inspection and setup checks, and scanning is parallelized across worker threads. The initial scan runs in the background after the TUI opens.

## Data Flow

```
App::new()
  → spawn background scan_projects()
  → render root immediately
  → apply AsyncResult::Projects when scan finishes

App::open_project(idx)
  → load_home_data_local()   — local git/setup reads only
  → load_prompts()           — filesystem read
  → scan_setup()             — filesystem checks
  → spawn background Home hydrate (gh repo view + avatar)
  → defer Issues / Memories / Skills / MCP until tab visit
```

Background work returns through an internal async result channel that is polled from the main event loop. The UI renders loading states while Home, Issues, avatar, or project scans are in flight.

## Prompt System

Prompts are Markdown files. Placeholders use `{PLACEHOLDER}` syntax. `auto_values()` pre-fills `{REPO}`, `{BRANCH}`, `{ISSUE}` from app state. Remaining placeholders are filled interactively via `PromptState::Fill`. `extract_body()` strips the first fenced code block if present (used for copying just the template content).

## Pane Tab

Tab 8 is a reserved placeholder. The intent is to embed a child TUI (Yazi, Helix) via `tui-term` (PTY emulator widget). `Ctrl+W` is reserved for focus handoff between the pane and pemguin nav. Not yet implemented.
