#!/bin/sh
# skilltap local installer — builds from this checkout and installs.
#
# Usage:
#   scripts/install-local.sh              # build + copy to ~/.local/bin/skilltap
#   scripts/install-local.sh --link       # build + symlink (live updates on rebuild)
#   scripts/install-local.sh --no-build   # use existing release binary, skip rebuild
#   SKILLTAP_INSTALL=/usr/local/bin scripts/install-local.sh
set -e

# --- Resolve repo root (script lives in <root>/scripts/) ---

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INSTALL_DIR="${SKILLTAP_INSTALL:-$HOME/.local/bin}"
BINARY_NAME="skilltap"
BUILD=1
MODE="copy"

for arg in "$@"; do
  case "$arg" in
    --link|--symlink) MODE="link" ;;
    --no-build)       BUILD=0 ;;
    -h|--help)
      sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      printf "error: unknown argument: %s\n" "$arg" >&2
      exit 1
      ;;
  esac
done

# --- Colors ---

if [ -t 1 ]; then
  BOLD='\033[1m'; GREEN='\033[32m'; CYAN='\033[36m'; RED='\033[31m'; RESET='\033[0m'
else
  BOLD=''; GREEN=''; CYAN=''; RED=''; RESET=''
fi
info() { printf "${CYAN}%s${RESET}\n" "$1"; }
ok()   { printf "${GREEN}%s${RESET}\n" "$1"; }
err()  { printf "${RED}error:${RESET} %s\n" "$1" >&2; }

cd "$REPO_ROOT"

# --- Build ---

if [ "$BUILD" -eq 1 ]; then
  if ! command -v cargo >/dev/null 2>&1; then
    err "cargo is not on PATH — required to build skilltap"
    exit 1
  fi
  info "Building skilltap from $REPO_ROOT ..."
  cargo build --locked --release -p skilltap --target-dir "$REPO_ROOT/target"
  ok "Built target/release/skilltap"
else
  if [ ! -x "$REPO_ROOT/target/release/skilltap" ]; then
    err "no target/release/skilltap binary found and --no-build was passed"
    exit 1
  fi
  info "Using existing target/release/skilltap (skipping build)"
fi

SOURCE="$REPO_ROOT/target/release/skilltap"

# --- Install ---

mkdir -p "$INSTALL_DIR"
TARGET="$INSTALL_DIR/$BINARY_NAME"

# Remove anything (file or symlink) at the target so we don't overwrite
# someone else's install in confusing ways.
if [ -e "$TARGET" ] || [ -L "$TARGET" ]; then
  rm -f "$TARGET"
fi

if [ "$MODE" = "link" ]; then
  ln -s "$SOURCE" "$TARGET"
  ok "Symlinked $TARGET → $SOURCE"
  info "Future release builds will be picked up automatically."
else
  cp "$SOURCE" "$TARGET"
  chmod +x "$TARGET"
  ok "Installed $TARGET"
fi

# --- Verify + PATH hint ---

VERSION="$("$TARGET" --version 2>/dev/null || true)"
if [ -n "$VERSION" ]; then
  ok "$VERSION is ready"
else
  err "binary installed but '--version' did not respond — check $TARGET"
  exit 1
fi

case ":$PATH:" in
  *:"$INSTALL_DIR":*) ;;
  *)
    printf "\n"
    info "${BOLD}Note:${RESET} $INSTALL_DIR is not on your PATH."
    printf "  Add this to your shell profile:\n\n"
    printf "    export PATH=\"%s:\$PATH\"\n\n" "$INSTALL_DIR"
    ;;
esac
