# [Project Name] — Agent Context

> Copy this to the project root as AGENT.md and fill it in.
> Global context lives at /Users/josh/Projects/AGENT.md and CONSTITUTION.md.

## What This Project Is

One paragraph. What does it do, who uses it, what problem does it solve.

## Stack

Languages, frameworks, bundler, package manager, database, deployment target — whatever this project actually uses.

## Running Locally

```bash
# Start dev server (Vite+ project)
vp dev

# Run checks (format + lint + typecheck)
vp check

# Run tests
vp test

# Any other processes that need to run alongside it
```

## Key Files & Directories

```
src/
  components/   — ...
  pages/        — ...
  hooks/        — ...
```

## Current Focus

What is actively being worked on right now. Update this as work shifts.

## Gotchas

Things that aren't obvious from reading the code:

- Any quirks, workarounds, or non-obvious decisions
- Why something was done a certain way if it looks wrong

## Skills

Skills live in `.agents/skills/` (all agents) with `.claude/skills/` symlinks (Claude specifically).

Discover available skills:
```bash
npx skills add <owner/repo> --list
```

Install a skill for all agents — omit `-a` flag to target all agents automatically:
```bash
npx skills add <owner/repo> --skill <name> -y
```

Example — common skills by stack:
```bash
# Vite React (all projects)
npx skills add vercel-labs/agent-skills --skill vercel-react-best-practices -y
npx skills add vercel-labs/agent-skills --skill vercel-composition-patterns -y
npx skills add vercel-labs/agent-skills --skill web-design-guidelines -y

# Convex projects
npx skills add https://github.com/waynesutton/convexskills --skill convex -y

# Supabase projects
npx skills add supabase/agent-skills --skill supabase-postgres-best-practices -y
```

See what's installed:
```bash
npx skills list
```

Installed project skills:
- _none yet_

## Issue Workflow

Work is tracked as GitHub Issues. When assigned an issue:

1. Read this file and SPEC.md before touching any code
2. Read the issue in full: `gh issue view <number>`
3. Touch only files relevant to the issue — do not refactor unrelated code
4. Run `vp check` and `vp build` — both must pass before committing
5. Commit with: `fix: <description> (closes #<number>)`
6. Do not close the issue. Do not open a PR. Stop after the commit.

Claude reviews the commit and closes the issue if complete.

## Memory

Read `.memory/MEMORY.md` before starting work. It indexes project-scoped memory files — load only the ones relevant to the current task.

Write new learnings, decisions, and gotchas back to `.memory/` as you go. Update the index in `.memory/MEMORY.md`.

For global context that applies across all projects, read `~/.pemguin/memory/MEMORY.md`.

## Agent Responsibilities

- Keep this file current. If you change the stack, commands, or major features, update AGENT.md and README.md as part of the same task.
- Update `## Current Focus` at the start of each session to reflect what's actively being worked on.
- SPEC.md is the feature source of truth. If a feature ships or changes, update it.

## Spec

See SPEC.md for the feature checklist.
