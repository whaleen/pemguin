# Agent Storage Interface Docs

Reference documents for how each supported agent stores its data locally. These are the source of truth for the TUI's session/memory/skills reader layer.

## Supported Agents

| Agent | Binary | Sessions | Memory | Skills | Doc |
|-------|--------|----------|--------|--------|-----|
| Claude Code | `claude` | `~/.claude/projects/<encoded>/` JSONL | `~/.claude/projects/<encoded>/memory/` (auto-written) | `~/.claude/skills/` + plugins | [claude.md](claude.md) |
| Codex | `codex` | `~/.codex/sessions/YYYY/MM/DD/` JSONL | `~/.codex/memories/<repo-name>/` (auto-written) | `~/.codex/skills/` (own hierarchy) | [codex.md](codex.md) |
| Gemini CLI | `gemini` | `~/.gemini/tmp/<name>/chats/` JSON | `~/.gemini/GEMINI.md` (auto-written by `save_memory`) | `~/.gemini/skills/` + `~/.agents/skills/` | [gemini.md](gemini.md) |
| Pi | `pi` | `~/.pi/agent/sessions/<encoded>/` JSONL | none (in-session only) | `~/.pi/agent/skills/` + `~/.agents/skills/` + npm packages | [pi.md](pi.md) |

## Shared Ecosystem

Skills are cross-agent. See [shared.md](shared.md) for the `~/.agents/skills/` directory format, `SKILL.md` spec, `npx skills` toolchain, and per-agent behavior table.

## Maintenance Flow

When an agent updates its storage format:

1. **Validate** — run the validation checklist in the agent's doc against a real session on disk
2. **Update interface doc** — update the relevant `docs/agents/<agent>.md` with the new structure
3. **Update TUI** — adjust the reader function in `cli/src/lib.rs` that parses that agent's sessions/memory

Reader functions to update by agent:
- Claude: `claude_project_dirs()`, `resolve_sessions()`, `claude_memory_path()`
- Codex: `import_codex_sessions()`, `parse_codex_session()`, `codex_memory_dirs()`
- Gemini: `gemini_memory_dirs()` + new `import_gemini_sessions()` (not yet implemented)
- Pi: new `pi_project_dirs()`, `import_pi_sessions()` (not yet implemented)

## Path Encoding Summary

Each agent has its own convention for turning an absolute path into a directory name:

| Agent | Rule | Example (`/Users/josh/Projects/_foo`) |
|-------|------|---------------------------------------|
| Claude | each non-alphanumeric → `-` | `-Users-josh-Projects--foo` (v2) |
| Codex | date-bucketed; `cwd` field in session meta | n/a (scan + filter) |
| Gemini | `sha256(path)` hex | `02aa8978ef34...` |
| Pi | wrap in `--`, each `/` → `-` | `--Users-josh-Projects-_foo--` |

## Session Format Summary

| Agent | File format | First-line marker | Project match method |
|-------|-------------|-------------------|----------------------|
| Claude | JSONL | `type: "last-prompt"` | encoded path in dir name |
| Codex | JSONL | `type: "session_meta"` with `cwd` | scan + `cwd` compare |
| Gemini | JSON (single object) | top-level `sessionId` + `projectHash` | sha256 dir lookup |
| Pi | JSONL | `type: "session"` with `cwd` | encoded path in dir name |
