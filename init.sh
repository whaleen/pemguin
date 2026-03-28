#!/usr/bin/env bash
# project-scaffolding/init.sh
# Run from the project root you want to initialize.
# Usage: bash /Users/josh/Projects/project-scaffolding/init.sh

set -e

SCAFFOLDING_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(pwd)"

echo "Initializing: $PROJECT_DIR"
echo ""

# 1. Remove Vite+-generated AGENTS.md if present
if [ -f "$PROJECT_DIR/AGENTS.md" ]; then
  rm "$PROJECT_DIR/AGENTS.md"
  echo "  Removed AGENTS.md (Vite+ generated — replaced by AGENT.md)"
fi

# 2. Copy AGENT.md template (skip if exists)
if [ -f "$PROJECT_DIR/AGENT.md" ]; then
  echo "  AGENT.md already exists — skipping"
else
  cp "$SCAFFOLDING_DIR/templates/AGENT.md" "$PROJECT_DIR/AGENT.md"
  echo "  Created AGENT.md"
fi

# 2. Copy SPEC.md template (skip if exists)
if [ -f "$PROJECT_DIR/SPEC.md" ]; then
  echo "  SPEC.md already exists — skipping"
else
  cp "$SCAFFOLDING_DIR/templates/SPEC.md" "$PROJECT_DIR/SPEC.md"
  echo "  Created SPEC.md"
fi

# 3. Symlink CLAUDE.md → AGENT.md
if [ -e "$PROJECT_DIR/CLAUDE.md" ]; then
  echo "  CLAUDE.md already exists — skipping"
else
  ln -s AGENT.md "$PROJECT_DIR/CLAUDE.md"
  echo "  Symlinked CLAUDE.md → AGENT.md"
fi

# 4. Symlink GEMINI.md → AGENT.md
if [ -e "$PROJECT_DIR/GEMINI.md" ]; then
  echo "  GEMINI.md already exists — skipping"
else
  ln -s AGENT.md "$PROJECT_DIR/GEMINI.md"
  echo "  Symlinked GEMINI.md → AGENT.md"
fi

# 5. Copy docs skeleton (skip if exists)
if [ -d "$PROJECT_DIR/docs" ]; then
  echo "  docs/ already exists — skipping"
else
  cp -r "$SCAFFOLDING_DIR/docs" "$PROJECT_DIR/docs"
  echo "  Created docs/"
fi

# 6. Patch .gitignore
GITIGNORE="$PROJECT_DIR/.gitignore"
MARKER="# Agent dirs"

if [ -f "$GITIGNORE" ] && grep -q "$MARKER" "$GITIGNORE"; then
  echo "  .gitignore already patched — skipping"
else
  cat >> "$GITIGNORE" <<'EOF'

# Agent dirs
.agents/
.claude/
.kiro/
.vite-hooks/
skills-lock.json
CLAUDE.md
GEMINI.md
.memory/
EOF
  echo "  Patched .gitignore"
fi

# 7. Create .memory index
mkdir -p "$PROJECT_DIR/.memory"
if [ -f "$PROJECT_DIR/.memory/MEMORY.md" ]; then
  echo "  .memory/MEMORY.md already exists — skipping"
else
  cat > "$PROJECT_DIR/.memory/MEMORY.md" <<'EOF'
# Memory Index

Agent memory for this project. Read this first, then load only the files relevant to the current task.

> Format: `- [filename.md](filename.md) - one-line description`

<!-- add entries below as memories are created -->
EOF
  echo "  Created .memory/MEMORY.md"
fi

# 8. Create .prompts sample prompt
mkdir -p "$PROJECT_DIR/.prompts"
if [ -f "$PROJECT_DIR/.prompts/work-on-task.md" ]; then
  echo "  .prompts/work-on-task.md already exists — skipping"
else
  cp "$SCAFFOLDING_DIR/templates/prompts/work-on-task.md" "$PROJECT_DIR/.prompts/work-on-task.md"
  echo "  Created .prompts/work-on-task.md"
fi

echo ""
echo "Done. Next steps:"
echo "  1. Edit AGENT.md — fill in what this project is, stack, commands, gotchas"
echo "  2. Edit SPEC.md  — write out the feature list before touching code"
echo "  3. Install skills — see AGENT.md for install instructions"
