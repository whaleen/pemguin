# [Project Name] Spec

> One-line description of what this project is and who it's for.

## What This Is

Brief narrative — the problem being solved, who it's for, why it exists.

---

## Features

### [Feature Area 1]

Requirements stated as observable behaviors, not implementation details. Each requirement should be verifiable by looking at the running app.

- Submitting an empty form shows a red border on required fields
- Deleting the last item shows the empty state illustration, not a blank screen
- If a fetch fails, an inline error appears — not a toast

### [Feature Area 2]

- ...

---

## Data & State

- What persists (localStorage, database, URL params, in-memory only)
- What resets on page refresh
- Any sync requirements between views or components

---

## Out of Scope

Things explicitly NOT part of this project to prevent scope creep:

- ...

---

## Docs

Detailed behavioral specs, architecture notes, and feature deep-dives live in `docs/`. Each feature area in this spec should have a corresponding doc if it needs more than a few bullet points.

- `docs/features/` — cross-cutting features
- `docs/architecture/` — system design and data flow
- `docs/<appname>/` — per-view behavioral specs

---

## Tracking

Requirements are tracked as GitHub Issues. SPEC.md is the source of truth — issues are the execution units.

### Creating an issue from this spec

```bash
gh issue create --title "[View] requirement" --body "$(cat <<'EOF'
## What
<what to build or change>

## Acceptance Criteria
- <observable behavior 1>
- <observable behavior 2>

## Scope
<files or views to touch — be explicit about what NOT to touch>
EOF
)"
```

### Issue workflow

1. Create the issue from a spec requirement
2. Hand to an agent with the prompt in `project-scaffolding/prompts/work-on-issue.md`
3. Agent works, runs `vp check` + `vp build`, commits with `closes #N`
4. Bring back to Claude to review the diff and close the issue if complete
