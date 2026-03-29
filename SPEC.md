# pemguin — Spec

Feature source of truth. Update this when features ship, change, or get cut.

## Projects List

- ✅ Scan projects directory up to 2 levels deep for `.git` dirs
- ✅ Group projects by parent directory name
- ✅ Show repo name, branch, dirty status, commits ahead
- ✅ Responsive repo name column width
- ✅ Rescan on `r`
- ✅ GitHub metadata sync on `s` (description, topics)
- ✅ Org avatar rendering via chafa (cached)
- ✅ Enter to open a project
- 📋 Filter / search projects by name
- 📋 Pin frequently used projects
- 📋 3-level scan depth (opt-in)

## Project Home Tab

- ✅ GitHub description, homepage URL, repo URL
- ✅ Edit description and homepage via `gh repo edit`
- ✅ Copy URL to clipboard with `y`
- ✅ Stack field (from `.pemguin.toml` or metadata)
- ✅ Topics from GitHub metadata
- ✅ Setup score (N/M configured)
- ✅ Recent commits list
- ✅ Org avatar display
- 📋 Git status summary (dirty file count, ahead/behind remote)
- 📋 Stash count

## Issues Tab

- ✅ List open GitHub issues via `gh`
- ✅ Issue title, number, labels
- ✅ Issue body preview
- ✅ Copy issue prompt to clipboard on enter
- 📋 Create issue from within pm
- 📋 Close / comment on issue

## Config Tab

- ✅ Check for AGENT.md / CLAUDE.md
- ✅ Check for SPEC.md
- ✅ Check for .mcp.json
- ✅ Check for skills-lock.json
- ✅ Check for stale AGENTS.md (old format)
- ✅ Apply missing items on enter
- ✅ Edit managed items via `e`
- ✅ Delete managed items via `d`
- ✅ Reset managed items to pemguin defaults via `R`
- ✅ Apply all on `a`
- ✅ Rescan on `r`
- 📋 Check for CONSTITUTION.md symlink
- 📋 Check for .memory/ directory

## Prompts Tab

- ✅ Browse global prompts (`~/.pemguin/prompts/`)
- ✅ Browse project prompts (`.prompts/`)
- ✅ Preview prompt content
- ✅ Fill in placeholders interactively
- ✅ Auto-fill `{REPO}`, `{ISSUE}`, `{BRANCH}` from context
- ✅ Copy filled prompt to clipboard
- 📋 Create new prompt from within pm
- 📋 Edit prompt from within pm

## Memories Tab

- ✅ Browse project memory (`.memory/`)
- ✅ Browse global memory (`~/.pemguin/memory/`)
- ✅ Browse Claude memory (`.claude/.../memory/`)
- ✅ Preview memory file content
- ✅ Create new memory file (prompts for name, opens `$EDITOR`)
- ✅ Edit existing memory file in `$EDITOR`
- ✅ Delete memory file with `d`
- ✅ Migrate Claude memory file to `.memory/` with `m`
- ✅ Reload on `r`

## Skills Tab

- ✅ Read installed skills from `skills-lock.json`
- ✅ Show skill name, source repo, description
- 📋 Install skill from within pm
- 📋 Remove skill from within pm

## MCP Tab

- ✅ Read configured servers from `.mcp.json`
- ✅ Show server name, command, args
- 📋 Add / edit / remove MCP server from within pm

## Sessions Tab (tab 9)

- ✅ List agent sessions per project (Claude Code, Codex, Gemini placeholder)
- ✅ Show agent type, start date, first user message, pending/resolved status
- ✅ Export indicator (`↓`) and pending indicator (`·`) in list
- ✅ New session picker — select agent + prompt, copy launch command to clipboard
- ✅ Resume session — copy resume command to clipboard with `y` (when ID resolved)
- ✅ Inline session summary view with `s` (full turn-by-turn JSONL render)
- ✅ Export session to `.pemguin/exports/` in markdown and plain text with `e`
- ✅ Delete session entry with `d`
- ✅ Import Claude sessions from `~/.claude/projects/` (both v1 `_`-preserving and v2 `_`→`-` encodings)
- ✅ Import Codex sessions from `~/.codex/sessions/YYYY/MM/DD/` matching `cwd`
- ✅ Session IDs are nullable — present when resolved, absent when pending (portable across machines)
- ✅ Persist session registry to `.pemguin/sessions.toml`
- 📋 Cache Codex session scan results — currently does a full walk of `~/.codex/sessions/YYYY/MM/DD/` on every Sessions tab open; for large Codex histories this can be slow. Future: persist last-scanned state and do incremental updates.
- 📋 Gemini session import
- 📋 Show session token count / cost estimate
- 📋 Filter sessions by agent type

## Pane Tab (tab 8)

- ✅ Launch `lazygit`, `yazi`, and `$EDITOR` in the project root
- 📋 Embedded child TUI via `tui-term` PTY
- 📋 Yazi file browser as first child
- 📋 `Ctrl+W` to toggle focus between pane and pm nav
- 📋 Session persistence (child stays alive when switching tabs)

## Navigation & UX

- ✅ Number keys 1–9 to switch tabs
- ✅ Tab key cycles through tabs
- ✅ Esc = back (InProject → Projects)
- ✅ Split header: identity row (badge + project + branch) + nav row (tabs)
- ✅ Footer hints update per-screen and per-mode
- ✅ Earthy color theme (Ratatui Color::Rgb palette)
- ✅ Nerd Font icons throughout
- ✅ Sidebar hidden below 70 col
- 📋 Mouse support
- 📋 Configurable color theme

## Configuration

- ✅ `~/.pemguin.toml` — projects root, future options
- ✅ `PEMGUIN_PROJECTS_DIR` env var override
- 📋 Per-project config overrides
- 📋 Custom scan depth

## Known Issues

- Full rescan resets list selection instead of preserving the current project focus
- `gh` errors surface as status messages but don't retry
- Pane tab is non-functional (placeholder only)
- Nerd Font glyphs render as boxes in terminals without Nerd Font support
