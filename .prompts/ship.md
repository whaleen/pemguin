# Ship pemguin

Execute a full release. Do not ask for confirmation at each step. Do it all.

## Steps — run in this order, no pausing between them

1. **Check current version** — read `cli/Cargo.toml`, note the current version
2. **Determine next version** — if the user specified a version, use it. Otherwise bump patch (X.Y.Z → X.Y.Z+1) and tell the user what you're bumping to
3. **Stage and commit all pending changes** — `git add -A && git commit -m "chore: <summary of changes>"` — write a real commit message based on what's in the diff, not a placeholder
4. **Bump version in `cli/Cargo.toml`** — update the `version` field under `[package]`
5. **Commit the version bump** — `git commit -am "chore: bump to vX.X.X"`
6. **Push commits** — `git push origin master`
7. **Tag** — `git tag vX.X.X`
8. **Push tag** — `git push origin vX.X.X` — this triggers GitHub Actions
9. **Confirm the workflow started** — `gh run list --repo whaleen/pemguin --limit 3`
10. **Report done** — tell the user the version, the tag, and the Actions URL to watch

## Rules

- Do not ask "should I proceed?" between steps
- Do not skip the Actions confirmation at the end
- If there are no uncommitted changes, skip steps 3 and write a standalone version bump commit
- If step 3 would produce an empty commit, skip it
- The release workflow builds both arch binaries, uploads to GitHub Releases, and updates the Homebrew formula automatically — you do not need to do any of that manually

## What the pipeline does (for your reference, not your job)

Once the tag is pushed, GitHub Actions handles:
- Building `aarch64-apple-darwin` and `x86_64-apple-darwin` binaries
- Uploading tarballs + sha256s to GitHub Releases
- Cloning `whaleen/homebrew-tap` and updating `Formula/pemguin.rb` with the new version and sha256s

`brew upgrade pemguin` will work once the workflow completes (~5 min).
