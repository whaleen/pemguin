# Release — pemguin

## Critical: how this pipeline works

This is a Rust TUI, NOT a Tauri app. Do NOT use lipo. Do NOT build a universal
binary. Build separate aarch64 and x86_64 binaries on matching runners and
distribute them as separate archives. Homebrew selects the right one at install
time via `Hardware::CPU.arm?`.

## Project details

| Field        | Value                            |
|--------------|----------------------------------|
| Binary name  | `pm` (the installed binary name) |
| Crate name   | `pemguin`                        |
| GitHub org   | `whaleen`                        |
| Tap repo     | `whaleen/homebrew-tap`           |
| Formula path | `Formula/pemguin.rb`             |
| Cargo.toml   | `cli/Cargo.toml`                 |

## Version bump

Only one place: `cli/Cargo.toml` — `version` under `[package]`.

## Release trigger

~~~
git tag vX.X.X && git push origin vX.X.X
~~~

## Workflow structure

3 jobs: `build` (matrix) → `update-tap` (depends on build).

### build (matrix)
~~~yaml
strategy:
  matrix:
    include:
      - os: macos-14
        target: aarch64-apple-darwin
      - os: macos-15-intel
        target: x86_64-apple-darwin
steps:
  - uses: actions/checkout@v4
  - run: rustup target add ${{ matrix.target }}
  - run: cargo build --release --locked --target ${{ matrix.target }}
  - run: |
      mkdir -p dist
      cp target/${{ matrix.target }}/release/pm dist/pm
      tar -C dist -czf pemguin-${RELEASE_TAG}-${{ matrix.target }}.tar.gz pm
  - run: shasum -a 256 pemguin-${RELEASE_TAG}-${{ matrix.target }}.tar.gz > pemguin-${RELEASE_TAG}-${{ matrix.target }}.tar.gz.sha256
  - uses: softprops/action-gh-release@v2
    with:
      tag_name: ${{ env.RELEASE_TAG }}
      files: |
        pemguin-${{ env.RELEASE_TAG }}-${{ matrix.target }}.tar.gz
        pemguin-${{ env.RELEASE_TAG }}-${{ matrix.target }}.tar.gz.sha256
~~~

### update-tap
~~~yaml
update-tap:
  needs: build
  runs-on: ubuntu-latest
  steps:
    - name: Download checksums
      run: |
        gh release download ${RELEASE_TAG} --pattern "*.sha256" --repo ${{ github.repository }}
      env:
        GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}

    - name: Parse sha256 values
      run: |
        ARM_SHA=$(cat pemguin-${RELEASE_TAG}-aarch64-apple-darwin.tar.gz.sha256 | awk '{print $1}')
        X86_SHA=$(cat pemguin-${RELEASE_TAG}-x86_64-apple-darwin.tar.gz.sha256 | awk '{print $1}')
        VERSION=${RELEASE_TAG#v}
        echo "ARM_SHA=$ARM_SHA" >> $GITHUB_ENV
        echo "X86_SHA=$X86_SHA" >> $GITHUB_ENV
        echo "VERSION=$VERSION" >> $GITHUB_ENV

    - name: Update Homebrew tap
      run: |
        git clone https://x-access-token:${{ secrets.GH_PAT }}@github.com/whaleen/homebrew-tap.git
        cd homebrew-tap
        sed -i "s/version \".*\"/version \"${VERSION}\"/" Formula/pemguin.rb
        sed -i "s/sha256 \"[^\"]*\" # aarch64/sha256 \"${ARM_SHA}\" # aarch64/" Formula/pemguin.rb
        sed -i "s/sha256 \"[^\"]*\" # x86_64/sha256 \"${X86_SHA}\" # x86_64/" Formula/pemguin.rb
        git config user.email "actions@github.com"
        git config user.name "GitHub Actions"
        git add Formula/pemguin.rb
        git commit -m "bump pemguin to ${VERSION}"
        git push
~~~

## Formula shape (Formula/pemguin.rb in whaleen/homebrew-tap)

~~~ruby
class Pemguin < Formula
  desc "Terminal project manager TUI"
  homepage "https://github.com/whaleen/pemguin"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/whaleen/pemguin/releases/download/v#{version}/pemguin-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER" # aarch64
    else
      url "https://github.com/whaleen/pemguin/releases/download/v#{version}/pemguin-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER" # x86_64
    end
  end

  def install
    bin.install "pm"
  end
end
~~~

The `# aarch64` and `# x86_64` comments are required for the sed replacements.

## Required secrets
- `GH_PAT` in whaleen/pemguin repo settings (Settings → Secrets → Actions)
- `GITHUB_TOKEN` is automatic — do not add it manually

## Current state
`.github/workflows/release.yml` exists and is active.
`Formula/pemguin.rb` exists in `whaleen/homebrew-tap`.
`GH_PAT` secret is configured in whaleen/pemguin repo settings.
Pipeline is live — push a tag to trigger a release.
