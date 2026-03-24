# Prompt: Deploy to Production

Use this prompt when pushing a commit (or set of commits) to production and verifying the Vercel build.

Replace `{REPO}` with the `owner/repo` slug.

---

## Before Handing to an Agent

- **Repo must be in a clean state** — all work committed
- Run `git status` to confirm before starting

## Prompt

```
Deploy the current main branch of {REPO} to production and verify the build.

Steps:
1. Run git status — confirm the repo is clean and on main
2. Run git push origin main
3. After push, run: vercel ls to get the latest deployment URL
4. Run: vercel inspect <url> — wait until status is Ready or Error
5. If Error: run vercel logs <url> and report the full build error
6. If Ready: report the deployment URL and confirm success

Do not make any code changes. If the build fails, report the logs and stop — do not attempt fixes.
```

---

## On Build Failure (Claude)

If the agent reports a build error, bring it to Claude with the logs:

```
The Vercel build failed on {REPO}. Here are the logs:

<paste logs>

Identify the cause and fix it.
```

Claude will:
- Read the error
- Identify the relevant file(s)
- Fix the issue and commit
- Hand back to the deploy prompt to retry
