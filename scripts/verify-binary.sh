#!/bin/sh
# Smoke-test the compiled skilltap binary without touching user configuration.
# Usage: scripts/verify-binary.sh [path-to-binary]
set -eu

if [ "$#" -gt 1 ]; then
  printf 'error: expected at most one binary path\n' >&2
  exit 2
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BINARY=${1:-"$REPO_ROOT/target/release/skilltap"}

case "$BINARY" in
  /*) ;;
  *) BINARY="$REPO_ROOT/$BINARY" ;;
esac

if [ ! -x "$BINARY" ]; then
  printf 'error: binary not found or not executable: %s\n' "$BINARY" >&2
  exit 2
fi

VERSION=$(awk -F '"' '/^version = "/ { print $2; exit }' "$REPO_ROOT/Cargo.toml")
if [ -z "$VERSION" ]; then
  printf 'error: workspace version is missing from Cargo.toml\n' >&2
  exit 2
fi

EXPECTED_VERSION="skilltap $VERSION"
if VERSION_OUTPUT=$("$BINARY" --version 2>&1); then
  :
else
  STATUS=$?
  printf 'error: --version failed with exit code %s: %s\n' "$STATUS" "$VERSION_OUTPUT" >&2
  exit 1
fi

if [ "$VERSION_OUTPUT" != "$EXPECTED_VERSION" ]; then
  printf 'error: unexpected --version output: expected "%s", got "%s"\n' \
    "$EXPECTED_VERSION" "$VERSION_OUTPUT" >&2
  exit 1
fi

if HELP_OUTPUT=$("$BINARY" --help 2>&1); then
  :
else
  STATUS=$?
  printf 'error: --help failed with exit code %s: %s\n' "$STATUS" "$HELP_OUTPUT" >&2
  exit 1
fi

if ! printf '%s\n' "$HELP_OUTPUT" | grep -Fq 'Usage: skilltap'; then
  printf 'error: --help output does not contain "Usage: skilltap"\n' >&2
  exit 1
fi

printf 'verified %s (%s)\n' "$BINARY" "$EXPECTED_VERSION"
