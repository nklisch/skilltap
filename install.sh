#!/bin/sh
# skilltap installer — https://github.com/nklisch/skilltap
# Usage: curl -fsSL https://raw.githubusercontent.com/nklisch/skilltap/main/install.sh | sh
set -e

REPO="nklisch/skilltap"
INSTALL_DIR="${SKILLTAP_INSTALL:-$HOME/.local/bin}"
BINARY_NAME="skilltap"

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

# --- HTTP helper (curl with wget fallback) ---

fetch() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$1"
  else
    err "curl or wget is required"
    exit 1
  fi
}

download() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$2" "$1"
  else
    err "curl or wget is required"
    exit 1
  fi
}

verify_checksum() {
  CHECKSUMS_FILE="$1"
  ARTIFACT="$2"
  ARTIFACT_NAME="$3"

  EXPECTED="$(awk -v artifact="$ARTIFACT_NAME" '$2 == artifact { print $1; found = 1; exit } END { if (!found) exit 1 }' "$CHECKSUMS_FILE")" || {
    err "Release checksums do not contain ${ARTIFACT_NAME}"
    exit 1
  }

  case "$EXPECTED" in
    ''|*[!0-9a-fA-F]*)
      err "Release checksum for ${ARTIFACT} is malformed"
      exit 1
      ;;
  esac
  if [ "$(printf '%s' "$EXPECTED" | awk '{ print length }')" -ne 64 ]; then
    err "Release checksum for ${ARTIFACT} is malformed"
    exit 1
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL="$(sha256sum "$ARTIFACT" | awk '{ print $1 }')"
  elif command -v shasum >/dev/null 2>&1; then
    ACTUAL="$(shasum -a 256 "$ARTIFACT" | awk '{ print $1 }')"
  else
    err "sha256sum or shasum is required to verify release artifacts"
    exit 1
  fi

  EXPECTED="$(printf '%s' "$EXPECTED" | tr '[:upper:]' '[:lower:]')"
  ACTUAL="$(printf '%s' "$ACTUAL" | tr '[:upper:]' '[:lower:]')"
  if [ "$ACTUAL" != "$EXPECTED" ]; then
    err "Checksum verification failed for ${ARTIFACT}"
    exit 1
  fi
}

# --- Detect platform ---

detect_platform() {
  OS="$(uname -s)"
  ARCH="$(uname -m)"

  case "$OS" in
    Linux*)  OS="linux" ;;
    Darwin*) OS="darwin" ;;
    *)       err "Unsupported OS: $OS"; exit 1 ;;
  esac

  case "$ARCH" in
    x86_64|amd64)  ARCH="x64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *)             err "Unsupported architecture: $ARCH"; exit 1 ;;
  esac
}

# --- Discover latest version ---

get_latest_version() {
  info "Fetching latest release..."
  # GitHub API returns JSON; extract tag_name without jq
  RESPONSE="$(fetch "https://api.github.com/repos/${REPO}/releases/latest")"
  VERSION="$(printf '%s' "$RESPONSE" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')"

  if [ -z "$VERSION" ]; then
    err "Could not determine latest version. Check https://github.com/${REPO}/releases"
    exit 1
  fi
}

# --- Main ---

main() {
  bold "skilltap installer"
  echo ""

  detect_platform
  get_latest_version

  ASSET="skilltap-${OS}-${ARCH}"
  URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
  CHECKSUMS_URL="https://github.com/${REPO}/releases/download/${VERSION}/checksums.txt"

  info "Downloading ${ASSET} (${VERSION})..."
  TMPFILE="$(mktemp)"
  CHECKSUMS_FILE="$(mktemp)"
  trap 'rm -f "$TMPFILE" "$CHECKSUMS_FILE"' EXIT

  download "$URL" "$TMPFILE"
  download "$CHECKSUMS_URL" "$CHECKSUMS_FILE"
  verify_checksum "$CHECKSUMS_FILE" "$TMPFILE" "$ASSET"
  ok "Verified ${ASSET} checksum"

  # Install
  mkdir -p "$INSTALL_DIR"
  mv "$TMPFILE" "${INSTALL_DIR}/${BINARY_NAME}"
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

  ok "Installed skilltap ${VERSION} to ${INSTALL_DIR}/${BINARY_NAME}"
  echo ""

  # Delegate harness detection and first-party plugin setup to the verified
  # Rust boundary. Binary availability and optional harness attention remain
  # separate, so an unsupported target never hides a valid install.
  BOOTSTRAP_STATUS=0
  BOOTSTRAP_RESULT="$(SKILLTAP_INSTALL="${INSTALL_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}" bootstrap --target all --json 2>&1)" || BOOTSTRAP_STATUS=$?
  if [ -n "$BOOTSTRAP_RESULT" ]; then
    printf '%s\n' "$BOOTSTRAP_RESULT"
  fi
  if [ "$BOOTSTRAP_STATUS" -ne 0 ] && [ "$BOOTSTRAP_STATUS" -ne 2 ]; then
    err "skilltap bootstrap failed before harness setup; rerun: ${INSTALL_DIR}/${BINARY_NAME} bootstrap --help"
    exit "$BOOTSTRAP_STATUS"
  fi

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
        fish) PROFILES="" ;;  # fish uses a different mechanism
        *)    PROFILES="$HOME/.profile" ;;
      esac

      for PROFILE in $PROFILES; do
        # Only update profiles that already exist or are the primary one
        if [ -f "$PROFILE" ] || [ "$PROFILE" = "$HOME/.profile" ]; then
          # Skip if already present
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
        # fish or unknown shell — print manual instructions
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
