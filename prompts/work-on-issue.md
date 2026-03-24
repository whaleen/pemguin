# Prompt: Work on Issue

Use this prompt when assigning an agent to work on a GitHub issue.

Replace `{ISSUE}` with the issue number and `{REPO}` with the `owner/repo` slug.

---

## Before Handing to an Agent

- **Repo must be in a clean state** — commit or stash any uncommitted work first
- Run `git status` to confirm before starting

## Prompt

```
Work on issue #{ISSUE} in {REPO}.

Before writing any code:
1. Read AGENT.md and SPEC.md in the project root
2. Read the issue in full: gh issue view {ISSUE}
3. Identify only the files relevant to the issue — do not touch anything else

Do the work. Then:
1. Run vp check — fix any errors before committing
2. Run vp build — must succeed
3. Commit with a message referencing the issue: "fix: <description> (closes #{ISSUE})"

Do not close the issue. Do not open a PR. Stop after the commit.
```

---

## Review Step (Claude)

After the agent commits, bring it back to Claude:

```
Review the work done on issue #{ISSUE} in {REPO} and close it if complete.
```

Claude will:
- Read the diff
- Verify acceptance criteria from the issue are met
- Run vp check if needed
- Close the issue or flag what's missing
