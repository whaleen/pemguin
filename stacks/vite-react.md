# Stack: Vite+ React

Base stack for all Vite-based React projects.

## Tools

- **Runtime**: Node.js (managed by Vite+)
- **Package manager**: pnpm (Vite+ default)
- **Toolchain**: [Vite+](https://viteplus.dev) — single `vite.config.ts` for dev, build, test, lint, fmt
- **Framework**: React 19 + TypeScript
- **Styling**: Tailwind CSS + shadcn/ui
- **Language**: TypeScript (strict)

## Key Commands

```bash
vp dev           # dev server
vp build         # production build
vp check         # format + lint + typecheck in one pass
vp check --fix   # auto-fix
vp test          # run tests
vp test watch    # watch mode
vp run <script>  # run a package.json script
```

## Config Conventions

- All tooling config in `vite.config.ts` — import from `vite-plus` not `vite`
- No separate `vitest.config.ts`, `.eslintrc`, `.prettierrc`, `lint-staged.config.*`
- Tests import from `vite-plus/test` not `vitest`
- Commit hooks via `vp config` → `.vite-hooks/`

## Project Structure

```
src/
  components/    — shared/reusable UI
  pages/         — route-level components
  hooks/         — custom React hooks
  lib/           — utilities, helpers
  types/         — shared TypeScript types
public/
vite.config.ts   — all tooling config lives here
```

## Testing

- Test runner: Vitest (via Vite+)
- Import from `vite-plus/test` not `vitest`
- Tests live alongside source: `src/foo.test.ts` next to `src/foo.ts`
- Run: `vp test` (single run) or `vp test watch` (watch mode)
- Write tests alongside new features — not after, not only when things break

## Skills

Copy `skills-lock.json` from the scaffolding dir to the project root, then:
```bash
npx skills update
```

Includes: `vercel-react-best-practices`, `vercel-composition-patterns`, `web-design-guidelines`

## Reference

See `_whaleen/warehouse` for a working example.
