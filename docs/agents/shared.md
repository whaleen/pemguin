# Shared Agent Ecosystem — `~/.agents/`

Skills are the one cross-agent standard. Claude Code, Gemini CLI, and Pi all read from `~/.agents/skills/`. Codex has its own separate skills directory.

**Validation command**: `ls ~/.agents/skills/` and `cat ~/.agents/.skill-lock.json`

---

## Directory Layout

```
~/.agents/
  skills/
    <skill-name>/
      SKILL.md            # skill definition (required)
      ...                 # optional assets, scripts, etc.
  .skill-lock.json        # registry of installed skills
```

---

## `SKILL.md` Format

A skill is a directory containing a `SKILL.md` file with YAML frontmatter:

```markdown
---
name: find-skills
description: Helps users discover and install skills when they ask questions like "how do I do X"...
---

# Skill Title

...skill instructions in plain Markdown...
```

The frontmatter `name` and `description` are used by agents when listing and selecting skills. The body is injected into the agent context when the skill is active.

---

## `~/.agents/.skill-lock.json`

Registry of globally installed skills. Version 3 format:

```json
{
  "version": 3,
  "skills": {
    "<skill-name>": {
      "source": "<owner>/<repo>",
      "sourceType": "github",
      "sourceUrl": "https://github.com/<owner>/<repo>.git",
      "skillPath": "skills/<name>/SKILL.md",
      "skillFolderHash": "<sha>",
      "installedAt": "<ISO8601>",
      "updatedAt": "<ISO8601>"
    }
  },
  "dismissed": {},
  "lastSelectedAgents": ["claude-code", "gemini-cli", "codex", ...]
}
```

`lastSelectedAgents` records which agent CLIs this skill installation was targeted for.

---

## `npx skills` Toolchain

Skills are installed and managed via `npx skills` (the open agent skills ecosystem):

```bash
npx skills find [query]           # search registry
npx skills add <owner/repo>       # install a skill
npx skills check                  # check for updates
npx skills update                 # update all skills
```

Skills install to `~/.agents/skills/<name>/` and are registered in `~/.agents/.skill-lock.json`.

---

## Per-Agent Behavior

| Agent | Reads `~/.agents/skills/`? | Own skills dir? | Notes |
|-------|---------------------------|-----------------|-------|
| Claude Code | Yes (user scope, mirrored via `~/.claude/skills/`) | `.claude/skills/` (project) | Lock file: `~/.agents/.skill-lock.json`; project lock: `.claude/skills-lock.json` |
| Gemini CLI | Yes (preferred over `~/.gemini/skills/` at same tier) | `~/.gemini/skills/` (user), `.gemini/skills/` (project) | `.agents/` takes precedence within each tier |
| Pi | Yes (global: `~/.agents/skills/`; project: `.agents/skills/`) | `~/.pi/agent/skills/` (user), `.pi/skills/` (project) | Root `.md` files ignored in `~/.agents/`; only `SKILL.md` dirs recognized |
| Codex | Partial (`.agents/skills/` in project dirs) | `~/.codex/skills/` (primary) | Does not read `~/.agents/skills/` globally; only project-level `.agents/skills/` |

---

## Project-Level Skills

Skills can also be installed at the project level:

- **Claude**: `<project>/.claude/skills-lock.json` + `<project>/.claude/skills/<name>/SKILL.md`
- **Pi**: via `pi install <source> -l` (local flag), stored in project settings
- **Gemini/Codex**: project-level skill scoping not observed

---

## SKILL.md Discovery Path (Agent Perspective)

When an agent starts, it walks:
1. Global: `~/.agents/skills/*/SKILL.md`
2. Project: `<cwd>/.claude/skills/*/SKILL.md` (Claude), or configured project paths
3. From packages: skills bundled inside npm packages (Pi only)

The agent injects enabled skills into its system prompt.
