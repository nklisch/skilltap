#!/bin/sh
# skilltap installer
# Usage: curl -fsSL https://raw.githubusercontent.com/nklisch/skilltap/main/scripts/install.sh | sh
# Or with options:
#   curl -fsSL ... | sh -s -- --dir ~/.local/bin
#   curl -fsSL ... | sh -s -- --version 0.2.0
set -e

REPO="nklisch/skilltap"
INSTALL_DIR="${SKILLTAP_INSTALL:-$HOME/.local/bin}"
VERSION=""

# --- Parse args ---

while [ $# -gt 0 ]; do
  case "$1" in
    --dir)     INSTALL_DIR="$2"; shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    *) printf "Unknown option: %s\n" "$1" >&2; exit 1 ;;
  esac
done

# --- Colors (only when stdout is a terminal) ---

if [ -t 1 ]; then
  BOLD='\033[1m'
  GREEN='\033[32m'
  CYAN='\033[36m'
  RED='\033[31m'
  RESET='\033[0m'
else
  BOLD='' GREEN='' CYAN='' RED='' RESET=''
fi

info()  { printf "${CYAN}%s${RESET}\n" "$1"; }
ok()    { printf "${GREEN}%s${RESET}\n" "$1"; }
err()   { printf "${RED}error:${RESET} %s\n" "$1" >&2; }
bold()  { printf "${BOLD}%s${RESET}\n" "$1"; }

# --- HTTP helpers ---

fetch() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$1"
  else
    err "curl or wget is required"; exit 1
  fi
}

download() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$2" "$1"
  else
    err "curl or wget is required"; exit 1
  fi
}

# --- Detect platform ---

detect_platform() {
  OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
  ARCH="$(uname -m)"

  case "$OS" in
    linux)  ;;
    darwin) ;;
    *)      err "Unsupported OS: $OS"; exit 1 ;;
  esac

  case "$ARCH" in
    x86_64|amd64)  ARCH="x64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *)             err "Unsupported architecture: $ARCH"; exit 1 ;;
  esac
}

# --- Resolve version ---

resolve_version() {
  if [ -z "$VERSION" ]; then
    info "Fetching latest release..."
    RESPONSE="$(fetch "https://api.github.com/repos/${REPO}/releases/latest")"
    VERSION="$(printf '%s' "$RESPONSE" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')"
    if [ -z "$VERSION" ]; then
      err "Could not determine latest version. Check https://github.com/${REPO}/releases"
      exit 1
    fi
  fi
}

# --- Verify checksum ---

verify_checksum() {
  BINARY_PATH="$1"
  CHECKSUMS_FILE="$2"
  BINARY_NAME="$3"

  if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
    info "Warning: sha256sum not found, skipping checksum verification"
    return
  fi

  EXPECTED="$(grep "$BINARY_NAME" "$CHECKSUMS_FILE" | awk '{print $1}')"
  if [ -z "$EXPECTED" ]; then
    err "Could not find checksum for $BINARY_NAME"
    exit 1
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL="$(sha256sum "$BINARY_PATH" | awk '{print $1}')"
  else
    ACTUAL="$(shasum -a 256 "$BINARY_PATH" | awk '{print $1}')"
  fi

  if [ "$EXPECTED" != "$ACTUAL" ]; then
    err "Checksum verification failed!"
    printf "  Expected: %s\n" "$EXPECTED" >&2
    printf "  Got:      %s\n" "$ACTUAL" >&2
    exit 1
  fi
}

# --- Main ---

main() {
  bold "skilltap installer"
  echo ""

  detect_platform
  resolve_version

  BINARY="skilltap-${OS}-${ARCH}"
  URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY}"
  CHECKSUM_URL="https://github.com/${REPO}/releases/download/${VERSION}/checksums.txt"

  info "Installing skilltap ${VERSION} (${OS}/${ARCH})..."

  TMPDIR="$(mktemp -d)"
  trap 'rm -rf "$TMPDIR"' EXIT

  download "$URL" "${TMPDIR}/skilltap"
  download "$CHECKSUM_URL" "${TMPDIR}/checksums.txt"

  verify_checksum "${TMPDIR}/skilltap" "${TMPDIR}/checksums.txt" "$BINARY"

  chmod +x "${TMPDIR}/skilltap"
  mkdir -p "$INSTALL_DIR"

  if [ -w "$INSTALL_DIR" ]; then
    mv "${TMPDIR}/skilltap" "${INSTALL_DIR}/skilltap"
  else
    sudo mv "${TMPDIR}/skilltap" "${INSTALL_DIR}/skilltap"
  fi

  # macOS: strip quarantine attribute
  if [ "$OS" = "darwin" ]; then
    xattr -d com.apple.quarantine "${INSTALL_DIR}/skilltap" 2>/dev/null || true
  fi

  ok "Installed skilltap ${VERSION} to ${INSTALL_DIR}/skilltap"
  echo ""

  # Update shell profiles if INSTALL_DIR isn't already on PATH
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      EXPORT_LINE="export PATH=\"${INSTALL_DIR}:\$PATH\""
      PROFILE_UPDATED=""

      # Build list of profile files to update based on current shell
      case "$(basename "${SHELL:-sh}")" in
        zsh)  PROFILES="$HOME/.zshrc $HOME/.profile" ;;
        bash) PROFILES="$HOME/.bashrc $HOME/.bash_profile $HOME/.profile" ;;
        fish) PROFILES="" ;;
        *)    PROFILES="$HOME/.profile" ;;
      esac

      for PROFILE in $PROFILES; do
        if [ -f "$PROFILE" ] || [ "$PROFILE" = "$HOME/.profile" ]; then
          if grep -qF "$INSTALL_DIR" "$PROFILE" 2>/dev/null; then
            PROFILE_UPDATED="$PROFILE"
            break
          fi
          printf '\n# skilltap\n%s\n' "$EXPORT_LINE" >> "$PROFILE"
          PROFILE_UPDATED="$PROFILE"
          break
        fi
      done

      if [ -n "$PROFILE_UPDATED" ]; then
        ok "Added to PATH in ${PROFILE_UPDATED}"
        echo ""
        bold "To use skilltap now, run:"
        echo ""
        echo "  source ${PROFILE_UPDATED}"
        echo ""
      else
        bold "Add skilltap to your PATH:"
        echo ""
        echo "  ${EXPORT_LINE}"
        echo ""
        echo "Then add that line to your shell profile."
      fi
      ;;
  esac
}

main
