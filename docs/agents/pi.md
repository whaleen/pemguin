# Pi — Agent Storage Interface

**Source**: github.com/badlogic/pi-mono, pi.dev/docs  
**Binary**: `pi` (`@mariozechner/pi-coding-agent`)  
**Config root**: `~/.pi/agent/` (overrideable via `PI_CODING_AGENT_DIR`)

---

## User-Level Directory Layout

```
~/.pi/agent/
  sessions/
    <encoded-path>/              # one dir per CWD
      <ISO8601>_<uuid>.jsonl    # conversation transcript
      subagent-artifacts/        # subagent I/O markdown files (if pi-subagents installed)
        <hex>_<agent>_<n>_input.md
        <hex>_<agent>_<n>_output.md
  skills/                        # user-level skills (root .md files are individual skills)
    <name>/
      SKILL.md                   # directory-based skill
    <single-file>.md             # root-level .md files also recognized as skills
  extensions/                    # user-level extensions (TypeScript)
    *.ts                         # single-file extensions
    <name>/
      index.ts                   # directory-based extension
  prompts/                       # global prompt templates
    <name>.md                    # /name command in the editor
  themes/                        # custom UI themes
  AGENTS.md                      # global context (or CLAUDE.md — both recognized)
  SYSTEM.md                      # global system prompt override
  settings.json                  # global settings
  auth.json                      # provider credentials
  models.json                    # custom models and providers
  keybindings.json               # keyboard shortcuts
  run-history.jsonl              # subagent run log (if pi-subagents installed)
  git/                           # git-based package installations
```

---

## Project-Level Directory Layout

```
<project-root>/
  AGENTS.md                      # project context (Pi reads this; also reads CLAUDE.md)
  CLAUDE.md                      # also read as project context
  .pi/
    settings.json                # project settings (overrides global; objects merged)
    SYSTEM.md                    # project system prompt (replaces default)
    APPEND_SYSTEM.md             # append to system prompt (instead of replacing)
    skills/                      # project-local skills
      <name>/
        SKILL.md
      <single-file>.md
    extensions/                  # project-local extensions (TypeScript)
      *.ts
      <name>/
        index.ts
    prompts/                     # project-local prompt templates
      <name>.md
    themes/                      # project-local themes
```

---

## What Pi Writes (Agent Default Behavior)

- `~/.pi/agent/sessions/<encoded-path>/<timestamp>_<uuid>.jsonl` — conversation transcripts
- `~/.pi/agent/auth.json` — credentials on first login
- `~/.pi/agent/run-history.jsonl` — subagent invocation log (if `pi-subagents` is installed)
- `~/.pi/agent/sessions/<encoded-path>/subagent-artifacts/` — subagent I/O files

---

## What Pi Reads (Context Discovery)

**Context files** (walked up from CWD to home/git-root):
1. `~/.pi/agent/AGENTS.md` or `~/.pi/agent/CLAUDE.md` — global context
2. `<project-root>/AGENTS.md` or `<project-root>/CLAUDE.md` — project context

**System prompt** (in priority order):
1. `.pi/SYSTEM.md` — project system prompt (replaces default)
2. `~/.pi/agent/SYSTEM.md` — global system prompt override
3. `.pi/APPEND_SYSTEM.md` / `~/.pi/agent/APPEND_SYSTEM.md` — append mode (doesn't replace)

**Skills** (auto-discovered in order):
1. `~/.pi/agent/skills/` — global user skills (root `.md` files and `SKILL.md` dirs)
2. `~/.agents/skills/` — shared agents directory (root `.md` ignored; `SKILL.md` dirs only)
3. `.pi/skills/` — project-local skills
4. `.agents/skills/` — project-shared skills (walked up through ancestors to git root)
5. npm/git packages — skills bundled in installed packages (e.g., `pi-subagents`)

**Extensions** (auto-discovered):
1. `~/.pi/agent/extensions/*.ts` and `~/.pi/agent/extensions/*/index.ts`
2. `.pi/extensions/*.ts` and `.pi/extensions/*/index.ts`
3. Packages declared in `settings.json` under `packages`

---

## Session File: `<ISO8601>_<uuid>.jsonl`

**Filename format**: `2026-04-26T19-28-13-092Z_<uuid>.jsonl` (colons replaced by dashes)

One JSON object per line. First line is always `type: "session"`.

**`session` line** (line 1):
```json
{
  "type": "session",
  "version": 3,
  "id": "<uuid>",
  "timestamp": "2026-04-26T19:28:13.092Z",
  "cwd": "/absolute/project/path"
}
```

Key line types: `session`, `message`, `model_change`, `thinking_level_change`, `compaction`

**`message` line**:
```json
{
  "type": "message",
  "id": "<short-hex>",
  "parentId": "<short-hex-or-null>",
  "timestamp": "...",
  "message": {
    "role": "user",
    "content": [ { "type": "text", "text": "..." } ]
  }
}
```

**`model_change` line** — Pi is model-agnostic; provider/model can change mid-session:
```json
{
  "type": "model_change",
  "provider": "google-gemini-cli",
  "modelId": "gemini-3.1-pro-preview"
}
```

---

## Path Encoding

Pi encodes the project CWD into the session directory name by wrapping the path in `--` and replacing `/` with `-`. Underscores are preserved.

`/Users/josh/Projects/_nothingdao/astrds` → `--Users-josh-Projects-_nothingdao-astrds--`

---

## Extensions (Packages)

Pi's extension system uses npm packages. Listed in `settings.json` under `packages`:

```json
{
  "packages": [
    "npm:pi-resource-center",
    "npm:pi-subagents"
  ]
}
```

Install: `pi install npm:<package>` or `pi install github:<owner>/<repo>`  
List: `pi list`  
Remove: `pi remove <source>`

Notable packages:
- `npm:pi-resource-center` — resource browser TUI (`/resource` command)
- `npm:pi-subagents` — subagent delegation (planner, reviewer, oracle, scout, worker, etc.)

---

## Prompt Templates

Stored at `~/.pi/agent/prompts/<name>.md` (global) or `.pi/prompts/<name>.md` (project). Filename without `.md` becomes the slash command (e.g., `review.md` → `/review`).

Optional YAML frontmatter: `description`, `argument-hint`.

---

## Settings: `settings.json`

Global at `~/.pi/agent/settings.json`; project at `.pi/settings.json`. Objects are merged (project overrides global). Paths resolve relative to their settings file location.

```json
{
  "defaultProvider": "google-gemini-cli",
  "defaultModel": "gemini-3.1-pro-preview",
  "defaultThinkingLevel": "medium",
  "packages": ["npm:pi-resource-center", "npm:pi-subagents"]
}
```

---

## Interface Validation Checklist

- [ ] `~/.pi/agent/sessions/` exists
- [ ] Encoded-path dir exists for target project
- [ ] Session files: `<ISO8601>_<uuid>.jsonl`, first line `type: "session"` with `cwd`
- [ ] `~/.pi/agent/settings.json` for active packages and default provider/model
- [ ] `~/.pi/agent/run-history.jsonl` for subagent history (only if pi-subagents installed)
- [ ] `AGENTS.md` or `CLAUDE.md` at project root (user-written; may not exist)
- [ ] `.pi/settings.json` for project-level overrides (may not exist)
- [ ] `~/.agents/skills/` for shared skills (check `.agents/skills/` within project too)
