#!/bin/sh
# skilltap binary smoke verifier — boots a compiled binary and runs critical
# commands against an isolated env. Catches the class of bug where `bun build
# --compile` produces a binary that fails at startup or on basic commands.
#
# Usage:
#   scripts/verify-binary.sh                 # verify ./skilltap (must already be built)
#   scripts/verify-binary.sh --build         # build first, then verify
#   scripts/verify-binary.sh path/to/binary  # verify a specific binary (e.g. skilltap-linux-x64)
#
# Exit codes:
#   0  all checks passed
#   1  a check failed
#   2  invalid arguments / missing binary
set -e

# --- Args ---

BUILD=0
BINARY=""

for arg in "$@"; do
  case "$arg" in
    --build) BUILD=1 ;;
    -h|--help)
      sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    -*)
      printf "error: unknown flag: %s\n" "$arg" >&2
      exit 2
      ;;
    *)
      if [ -n "$BINARY" ]; then
        printf "error: multiple binary paths given\n" >&2
        exit 2
      fi
      BINARY="$arg"
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

if [ -z "$BINARY" ]; then
  BINARY="$REPO_ROOT/skilltap"
fi

# Resolve to absolute path so isolated-env cd doesn't break it
case "$BINARY" in
  /*) ;;
  *)  BINARY="$REPO_ROOT/$BINARY" ;;
esac

# --- Colors ---

if [ -t 1 ]; then
  GREEN='\033[32m'; CYAN='\033[36m'; RED='\033[31m'; DIM='\033[2m'; RESET='\033[0m'
else
  GREEN=''; CYAN=''; RED=''; DIM=''; RESET=''
fi
info() { printf "${CYAN}%s${RESET}\n" "$1"; }
ok()   { printf "${GREEN}  ✓ %s${RESET}\n" "$1"; }
fail() { printf "${RED}  ✗ %s${RESET}\n" "$1" >&2; }
dim()  { printf "${DIM}    %s${RESET}\n" "$1"; }

# --- Build (optional) ---

if [ "$BUILD" -eq 1 ]; then
  info "Building skilltap..."
  bun run build
fi

if [ ! -x "$BINARY" ]; then
  fail "binary not found or not executable: $BINARY"
  exit 2
fi

info "Verifying $BINARY"

# --- Isolated env so smoke checks don't touch real config ---

TMP_HOME="$(mktemp -d)"
TMP_CFG="$(mktemp -d)"
cleanup() { rm -rf "$TMP_HOME" "$TMP_CFG"; }
trap cleanup EXIT INT TERM

export SKILLTAP_HOME="$TMP_HOME"
export XDG_CONFIG_HOME="$TMP_CFG"
export DO_NOT_TRACK=1            # silence telemetry prompt

FAILED=0

run_check() {
  label="$1"
  shift
  out="$("$BINARY" "$@" 2>&1)" || rc=$? && rc=${rc:-0}
  rc=${rc:-$?}
  printf "%s\n" "$out" > /tmp/skilltap-verify-out.$$ 2>/dev/null || true
  echo "$out"  # for debug; redirected by caller via tee
}

# 1. --version: must boot, exit 0, print a version
info "  Check 1/3: --version boots cleanly"
VERSION_OUT="$("$BINARY" --version 2>&1)" && VERSION_RC=0 || VERSION_RC=$?
if [ "$VERSION_RC" -ne 0 ]; then
  fail "--version exited with code $VERSION_RC"
  printf "%s\n" "$VERSION_OUT" | sed 's/^/    /' >&2
  FAILED=1
elif ! printf "%s" "$VERSION_OUT" | grep -Eq '[0-9]+\.[0-9]+\.[0-9]+'; then
  fail "--version did not print a semver-shaped version"
  dim "got: $VERSION_OUT"
  FAILED=1
else
  ok "--version → $(printf '%s' "$VERSION_OUT" | tr -d '\n')"
fi

# 2. --help: must mention USAGE and skilltap (exercises citty subcommand registration)
info "  Check 2/3: --help renders"
HELP_OUT="$("$BINARY" --help 2>&1)" && HELP_RC=0 || HELP_RC=$?
if [ "$HELP_RC" -ne 0 ]; then
  fail "--help exited with code $HELP_RC"
  printf "%s\n" "$HELP_OUT" | sed 's/^/    /' >&2
  FAILED=1
elif ! printf "%s" "$HELP_OUT" | grep -q "USAGE"; then
  fail "--help output did not contain 'USAGE'"
  dim "got first line: $(printf '%s' "$HELP_OUT" | head -n1)"
  FAILED=1
elif ! printf "%s" "$HELP_OUT" | grep -q "skilltap"; then
  fail "--help output did not mention 'skilltap'"
  FAILED=1
else
  ok "--help renders ($(printf '%s' "$HELP_OUT" | wc -l | tr -d ' ') lines)"
fi

# 3. doctor --json: exercises core paths (state, taps, agents, fs) end-to-end
info "  Check 3/3: doctor --json runs without crashing"
DOCTOR_OUT="$("$BINARY" doctor --json 2>&1)" && DOCTOR_RC=0 || DOCTOR_RC=$?
if [ "$DOCTOR_RC" -ne 0 ]; then
  fail "doctor --json exited with code $DOCTOR_RC"
  printf "%s\n" "$DOCTOR_OUT" | sed 's/^/    /' >&2
  FAILED=1
elif ! printf "%s" "$DOCTOR_OUT" | grep -q '"checks"'; then
  fail "doctor --json output missing 'checks' field"
  dim "got: $(printf '%s' "$DOCTOR_OUT" | head -c 200)"
  FAILED=1
else
  ok "doctor --json returned valid output"
fi

# --- Summary ---

printf "\n"
if [ "$FAILED" -eq 0 ]; then
  printf "${GREEN}✓ all binary smoke checks passed${RESET}\n"
  exit 0
else
  printf "${RED}✗ binary smoke verification failed${RESET}\n" >&2
  exit 1
fi
