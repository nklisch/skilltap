#!/bin/sh
# Check that public installation surfaces describe one bootstrap path without
# mutating the active sibling marketplace repository.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
GETTING_STARTED="$ROOT/website/guide/getting-started.md"
UPDATES="$ROOT/website/guide/updates.md"
README="$ROOT/README.md"
FORMULA="$ROOT/homebrew-skilltap/Formula/skilltap.rb"

grep -Fq 'Native marketplace' "$GETTING_STARTED" || { echo "error: website lacks marketplace installation path" >&2; exit 1; }
grep -Fq 'curl -fsSL https://skilltap.dev/install.sh | sh' "$GETTING_STARTED" || { echo "error: website lacks one-line installer path" >&2; exit 1; }
grep -Fq 'skilltap bootstrap' "$GETTING_STARTED" || { echo "error: website lacks bootstrap handoff" >&2; exit 1; }
grep -Fq -- '--allow-major' "$UPDATES" || { echo "error: website lacks major-update policy" >&2; exit 1; }
grep -Fq 'daemon' "$UPDATES" || { echo "error: website lacks daemon policy" >&2; exit 1; }
grep -Fq 'Native Claude Code or Codex marketplace' "$README" || { echo "error: README lacks marketplace parity" >&2; exit 1; }
grep -Fq 'skilltap bootstrap' "$README" || { echo "error: README lacks bootstrap handoff" >&2; exit 1; }
if grep -Eiq 'brew.*install.*plugin|homebrew.*install.*plugin' "$FORMULA"; then
  echo "error: Homebrew formula must not claim to install harness plugins" >&2
  exit 1
fi

# The active ../skills checkout is intentionally read-only here. CI can point
# at a checkout with SKILLTAP_SKILLS_MARKETPLACE; local absence is expected.
if [ -n "${SKILLTAP_SKILLS_MARKETPLACE:-}" ]; then
  SIBLING=$SKILLTAP_SKILLS_MARKETPLACE
  grep -Fq '"name": "skilltap"' "$SIBLING" || { echo "error: sibling marketplace has no skilltap entry" >&2; exit 1; }
  grep -Fq 'https://github.com/nklisch/skilltap' "$SIBLING" || { echo "error: sibling skilltap entry is not canonical" >&2; exit 1; }
  grep -Fq '"path": "plugin"' "$SIBLING" || { echo "error: sibling skilltap entry is not pointed at plugin/" >&2; exit 1; }
else
  echo "sibling marketplace check skipped (set SKILLTAP_SKILLS_MARKETPLACE in a parity checkout)"
fi

echo "installation surfaces verified"
