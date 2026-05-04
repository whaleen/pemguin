# Codex — Agent Storage Interface

**Source**: developers.openai.com/codex, github.com/openai/codex  
**Binary**: `codex`  
**Config root**: `~/.codex/` (overrideable via `CODEX_HOME`)

---

## User-Level Directory Layout

```
~/.codex/
  sessions/
    YYYY/MM/DD/
      rollout-<timestamp>-<uuid>.jsonl   # conversation transcript
  memories/                              # persistent memory (Codex-written)
    <project-name>/
  skills/
    .system/                             # built-in skills (Codex-provided)
      skill-creator/
        SKILL.md
      skill-installer/
        SKILL.md
        scripts/
      imagegen/
      openai-docs/
      plugin-creator/
    <user-installed-skill>/
      SKILL.md
  rules/
    default.rules                        # default Starlark rules (auto-written on approval)
  AGENTS.md                              # global context/instructions (user-written)
  AGENTS.override.md                     # overrides AGENTS.md (user-written, takes precedence)
  config.toml                            # global configuration
  auth.json                              # credentials
  history.jsonl                          # global command history
  hooks.json                             # user-level lifecycle hooks
  logs/                                  # debug logs
  state_5.sqlite                         # agent state database
  cache/
```

---

## Project-Level Directory Layout

```
<project-root>/
  AGENTS.md                  # project context/instructions (committed to git)
  AGENTS.override.md         # personal override (takes precedence over AGENTS.md)
  .codex/
    config.toml              # project configuration (only read when project is trusted)
    hooks.json               # project-level lifecycle hooks
    rules/                   # project-level Starlark rules
      *.rules
    skills/                  # project-level skills
      <name>/
        SKILL.md
```

---

## What Codex Writes (Agent Default Behavior)

- `~/.codex/sessions/YYYY/MM/DD/<name>.jsonl` — conversation transcripts organized by date
- `~/.codex/memories/<project-name>/` — persistent memory summaries from sessions
- `~/.codex/rules/default.rules` — approval rules, written interactively as user approves/denies tool calls
- `~/.codex/history.jsonl` — global command history
- `~/.codex/state_5.sqlite` — agent job state

---

## What Codex Reads (Context Discovery)

**AGENTS.md files** (walked from current directory up to project root):
1. `~/.codex/AGENTS.override.md` or `~/.codex/AGENTS.md` (global)
2. `<project-root>/AGENTS.override.md` or `<project-root>/AGENTS.md` (project)
3. Any `AGENTS.md` in subdirectories walked down to current working directory

`AGENTS.override.md` takes precedence over `AGENTS.md` at the same level.

**Skills** (discovery order):
1. `~/.codex/skills/.system/` — built-in system skills
2. `~/.codex/skills/<name>/` — user-installed skills
3. `.codex/skills/<name>/` — project-level skills
4. `.agents/skills/` — project-shared location (walked up to git root)

---

## Session File: `rollout-<timestamp>-<uuid>.jsonl`

One JSON object per line. First line is always `type: "session_meta"`.

**`session_meta` line**:
```json
{
  "timestamp": "...",
  "type": "session_meta",
  "payload": {
    "id": "<uuid>",
    "cwd": "/absolute/project/path",
    "originator": "codex-tui",
    "cli_version": "...",
    "model_provider": "openai"
  }
}
```

Key line types: `session_meta`, `event_msg`, `response_item`, `turn_context`

**Project matching**: Codex does NOT encode the project path in the directory name. All sessions are date-bucketed. To find sessions for a project, scan all files and filter by `session_meta.payload.cwd`. This is O(n) across all sessions.

---

## Memory System

`~/.codex/memories/` — Codex writes summaries and durable entries from session context. Organized by project name (the bare directory name, not the full path). These are auto-generated and should not be manually edited.

---

## Rules System (Starlark)

Rules are written in Starlark (a safe scripting language). They define what Codex is and is not allowed to do without prompting.

- User rules: `~/.codex/rules/*.rules`
- Project rules: `.codex/rules/*.rules` (only loaded when project is trusted)
- Default rules: written to `~/.codex/rules/default.rules` as the user approves/denies tool calls interactively

---

## Skills Format

Skills follow the `SKILL.md` format (see [shared.md](shared.md)). System skills at `~/.codex/skills/.system/` may additionally include `scripts/` (Python) and `agents/` (YAML) subdirectories.

Codex does NOT read from `~/.agents/skills/` by default — it uses its own `~/.codex/skills/` hierarchy. The `.agents/skills/` path in project directories is a cross-agent convention also supported.

---

## Configuration: `config.toml`

```toml
[history]
persistence = "save_all"    # or "none"
max_bytes = 10485760

[logging]
log_dir = "~/.codex/logs"

[projects."/absolute/path"]
trust_level = "trusted"
```

---

## Interface Validation Checklist

- [ ] `~/.codex/sessions/` exists with `YYYY/MM/DD/` structure
- [ ] First line of each JSONL has `type: "session_meta"` with `cwd` field
- [ ] `~/.codex/memories/` exists (may be empty)
- [ ] `~/.codex/skills/` exists with `.system/` subdirectory
- [ ] `AGENTS.md` or `AGENTS.override.md` at project root and `~/.codex/` (user-written; may not exist)
- [ ] `.codex/config.toml` for project trust level
- [ ] `~/.codex/rules/default.rules` for auto-written approval rules
