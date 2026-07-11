#!/bin/sh
# Run every compiled CLI contract against one explicit optimized binary.
# Usage: scripts/verify-compiled-binary.sh path-to-binary
set -eu

if [ "$#" -ne 1 ]; then
  printf 'error: expected exactly one binary path\n' >&2
  exit 2
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BINARY=$1

case "$BINARY" in
  /*) ;;
  *) BINARY="$(pwd)/$BINARY" ;;
esac

"$SCRIPT_DIR/verify-binary.sh" "$BINARY"

cd "$REPO_ROOT"
SKILLTAP_TEST_BIN="$BINARY" cargo test --locked -p skilltap --test compiled_binary
