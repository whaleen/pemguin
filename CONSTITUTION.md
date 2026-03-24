# Constitution

Universal principles that apply across all projects. These are not suggestions — agents must follow them exactly.

## Workflow

- **No feature branches** — commit directly to `main`
- **No dev/prod separation** — local dev is production
- **No git amend** — always new commits
- **No Claude mentions in commit messages**
- Solo developer — no PR review ceremony, no changelogs unless asked

## API Design

Any API consumed by more than one client, or callable by an external agent or tool, gets an OpenAPI spec. This gives agents an unambiguous contract to work from without grepping through route handlers.

- **Single-consumer internal APIs**: optional, use judgement
- **Platform-managed schemas** (Supabase, Convex, etc.): skip — the platform provides its own type contract

When a spec exists, read it before touching any API-related code.

## Code Philosophy

- **No over-engineering** — minimum complexity for the current task
- **No premature abstraction** — three similar lines beats a helper function
- **No speculative features** — build what is asked, nothing more
- **No backwards-compatibility shims** — just change the code
- **No error handling for impossible scenarios** — trust the framework
- **No comments unless logic is non-obvious**
- Prefer editing existing files over creating new ones

## Agent Behavior

- Responses should be short and direct — no preamble, no trailing summaries
- No emojis unless explicitly asked
- No time estimates
- If blocked, don't retry the same approach — diagnose or ask
- Always read a file before editing it
- Check `AGENT.md` in the project for local context
