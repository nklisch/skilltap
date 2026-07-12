#!/bin/sh
# Check that public installation surfaces lead with working native plugin
# commands and retain the standalone bootstrap path without mutating the active
# sibling marketplace repository.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
LANDING="$ROOT/website/index.md"
GETTING_STARTED="$ROOT/website/guide/getting-started.md"
UPDATES="$ROOT/website/guide/updates.md"
README="$ROOT/README.md"
FORMULA="$ROOT/homebrew-skilltap/Formula/skilltap.rb"

for surface in "$LANDING" "$GETTING_STARTED" "$README"; do
  grep -Fq 'claude plugin marketplace add nklisch/skilltap --scope user' "$surface" || { echo "error: $surface lacks the Claude marketplace shorthand" >&2; exit 1; }
  grep -Fq 'claude plugin install skilltap@skilltap --scope user' "$surface" || { echo "error: $surface lacks the Claude plugin install command" >&2; exit 1; }
  grep -Fq 'codex plugin marketplace add nklisch/skilltap' "$surface" || { echo "error: $surface lacks the Codex marketplace shorthand" >&2; exit 1; }
  grep -Fq 'codex plugin add skilltap@skilltap' "$surface" || { echo "error: $surface lacks the Codex plugin install command" >&2; exit 1; }
  grep -Fq 'curl -fsSL https://skilltap.dev/install.sh | sh' "$surface" || { echo "error: $surface lacks the one-line installer path" >&2; exit 1; }
  grep -Fq 'skilltap bootstrap' "$surface" || { echo "error: $surface lacks the bootstrap handoff" >&2; exit 1; }
done
grep -Fq -- '--allow-major' "$UPDATES" || { echo "error: website lacks major-update policy" >&2; exit 1; }
grep -Fq 'daemon' "$UPDATES" || { echo "error: website lacks daemon policy" >&2; exit 1; }
if grep -Eiq 'brew.*install.*plugin|homebrew.*install.*plugin' "$FORMULA"; then
  echo "error: Homebrew formula must not claim to install harness plugins" >&2
  exit 1
fi

# The active ../skills checkout is intentionally read-only here. CI can point
# at a checkout with SKILLTAP_SKILLS_MARKETPLACE; local absence is expected.
if [ -n "${SKILLTAP_SKILLS_MARKETPLACE:-}" ]; then
  SIBLING=$SKILLTAP_SKILLS_MARKETPLACE
  grep -Eq '"name"[[:space:]]*:[[:space:]]*"skilltap"' "$SIBLING" || { echo "error: sibling marketplace has no skilltap entry" >&2; exit 1; }
  grep -Fq 'https://github.com/nklisch/skilltap' "$SIBLING" || { echo "error: sibling skilltap entry is not canonical" >&2; exit 1; }
  grep -Eq '"path"[[:space:]]*:[[:space:]]*"plugin"' "$SIBLING" || { echo "error: sibling skilltap entry is not pointed at plugin/" >&2; exit 1; }
else
  echo "sibling marketplace check skipped (set SKILLTAP_SKILLS_MARKETPLACE in a parity checkout)"
fi

echo "installation surfaces verified"
