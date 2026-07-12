#!/bin/sh
# Static installer contract checks. This is intentionally side-effect free.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
INSTALLER=$ROOT/install.sh

grep -q 'sha256sum' "$INSTALLER" || { echo "installer must verify sha256sum" >&2; exit 1; }
grep -q 'shasum -a 256' "$INSTALLER" || { echo "installer must support macOS shasum" >&2; exit 1; }
grep -q 'bootstrap --target all --json' "$INSTALLER" || { echo "installer must delegate bootstrap" >&2; exit 1; }
grep -q 'SKILLTAP_INSTALL=' "$INSTALLER" || { echo "installer must pass the verified destination" >&2; exit 1; }

# The shell entry point may only invoke fixed release URLs and the verified
# binary. Reject common shell-evaluation and privilege-escalation regressions.
if grep -Eq 'eval|sudo|sh -c|bash -c' "$INSTALLER"; then
  echo "installer contains an unsafe shell execution path" >&2
  exit 1
fi

echo "installer contract checks passed"
