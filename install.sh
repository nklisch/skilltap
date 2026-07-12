#!/bin/sh
# skilltap installer — https://github.com/nklisch/skilltap
# Usage: curl -fsSL https://raw.githubusercontent.com/nklisch/skilltap/main/install.sh | sh
set -eu

REPO="nklisch/skilltap"
INSTALL_DIR="${SKILLTAP_INSTALL:-$HOME/.local/bin}"
BINARY_NAME="skilltap"
MAX_METADATA_BYTES=4194304
MAX_CHECKSUM_BYTES=1048576
MAX_ARTIFACT_BYTES=67108864

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

info()  { printf '%s%s%s\n' "$CYAN" "$1" "$RESET"; }
ok()    { printf '%s%s%s\n' "$GREEN" "$1" "$RESET"; }
err()   { printf '%serror:%s %s\n' "$RED" "$RESET" "$1" >&2; }
bold()  { printf '%s%s%s\n' "$BOLD" "$1" "$RESET"; }

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "$1 is required to install skilltap safely"
    exit 1
  fi
}

path_owner() {
  # GNU and BSD stat use different flags; fail closed if neither is available.
  stat -c '%u' "$1" 2>/dev/null || stat -f '%u' "$1" 2>/dev/null
}

validate_existing_path() {
  cursor="$1"
  while [ "$cursor" != "/" ] && [ -n "$cursor" ]; do
    if [ -L "$cursor" ]; then
      err "install path contains a symlink: $cursor"
      exit 1
    fi
    if [ -e "$cursor" ] && [ ! -d "$cursor" ]; then
      err "install path component is not a directory: $cursor"
      exit 1
    fi
    cursor=$(dirname "$cursor")
  done
}

validate_install_dir() {
  case "$INSTALL_DIR" in
    /*) ;;
    *) err "SKILLTAP_INSTALL must be an absolute user-owned directory"; exit 1 ;;
  esac
  case "$INSTALL_DIR" in
    *..*|*//*|*[[:cntrl:]]*)
      err "SKILLTAP_INSTALL contains an unsafe path component"
      exit 1
      ;;
  esac

  validate_existing_path "$INSTALL_DIR"
  if [ -e "$INSTALL_DIR" ] || [ -L "$INSTALL_DIR" ]; then
    [ -d "$INSTALL_DIR" ] || { err "SKILLTAP_INSTALL is not a directory"; exit 1; }
    [ -w "$INSTALL_DIR" ] || { err "SKILLTAP_INSTALL is not writable"; exit 1; }
    owner=$(path_owner "$INSTALL_DIR") || { err "could not verify install directory owner"; exit 1; }
    [ "$owner" = "$(id -u)" ] || { err "SKILLTAP_INSTALL must be owned by the current user"; exit 1; }
  else
    mkdir -p "$INSTALL_DIR" || { err "could not create SKILLTAP_INSTALL"; exit 1; }
    validate_existing_path "$INSTALL_DIR"
    owner=$(path_owner "$INSTALL_DIR") || { err "could not verify install directory owner"; exit 1; }
    [ "$owner" = "$(id -u)" ] || { err "SKILLTAP_INSTALL must be owned by the current user"; exit 1; }
  fi

  destination="${INSTALL_DIR}/${BINARY_NAME}"
  if [ -L "$destination" ]; then
    err "install destination must not be a symlink"
    exit 1
  fi
  if [ -e "$destination" ]; then
    [ -f "$destination" ] || { err "install destination is not a regular file"; exit 1; }
    owner=$(path_owner "$destination") || { err "could not verify install destination owner"; exit 1; }
    [ "$owner" = "$(id -u)" ] || { err "install destination must be owned by the current user"; exit 1; }
  fi
}

validate_release_tag() {
  printf '%s\n' "$1" | awk 'length($0) <= 64 && $0 ~ /^v[0-9]+[.][0-9]+[.][0-9]+$/ { found=1 } END { exit(found ? 0 : 1) }'
}

validate_effective_url() {
  case "$1" in
    https://github.com/*|https://api.github.com/*|https://objects.githubusercontent.com/*) ;;
    *) err "release download redirected to an untrusted host"; exit 1 ;;
  esac
}

validate_transfer() {
  metadata="$1"
  status=$(sed -n '1p' "$metadata")
  effective=$(sed -n '2p' "$metadata")
  case "$status" in
    2[0-9][0-9]) ;;
    *) err "release download returned an unexpected HTTP status"; exit 1 ;;
  esac
  [ -n "$effective" ] || { err "release download did not report its final URL"; exit 1; }
  validate_effective_url "$effective"
}

# --- HTTP helper (bounded direct downloads; no command construction) ---

download() {
  url="$1"
  output_path="$2"
  max_bytes="${3:-$MAX_ARTIFACT_BYTES}"
  metadata="${output_path}.meta"
  if command -v curl >/dev/null 2>&1; then
    curl --fail --silent --show-error --location \
      --max-redirs 3 --proto '=https' --proto-redir '=https' \
      --connect-timeout 10 --max-time 30 --max-filesize "$max_bytes" \
      --user-agent 'skilltap-installer/3' --output "$output_path" \
      --write-out '%{http_code}\n%{url_effective}\n' "$url" >"$metadata"
    validate_transfer "$metadata"
  elif command -v wget >/dev/null 2>&1; then
    # wget cannot expose the effective URL portably; its HTTPS-only bounded
    # redirect policy is constrained to the fixed release URL supplied here.
    wget --quiet --https-only --max-redirect=3 --timeout=30 --tries=1 \
      --output-document="$output_path" "$url"
    printf '200\n%s\n' "$url" >"$metadata"
  else
    err "curl or wget is required"
    exit 1
  fi
  bytes=$(wc -c <"$output_path" | tr -d '[:space:]')
  case "$bytes" in ''|*[!0-9]*) err "download size could not be verified"; exit 1 ;; esac
  [ "$bytes" -le "$max_bytes" ] || { err "release download exceeds the size limit"; exit 1; }
}

verify_checksum() {
  checksums_file="$1"
  artifact="$2"
  artifact_name="$3"
  bytes=$(wc -c <"$checksums_file" | tr -d '[:space:]')
  [ "$bytes" -le "$MAX_CHECKSUM_BYTES" ] || { err "release checksums exceed the size limit"; exit 1; }
  expected=$(awk -v artifact="$artifact_name" '
    $2 == artifact { if (found++) duplicate=1; value=$1 }
    END { if (duplicate || found != 1) exit 1; print value }
  ' "$checksums_file") || { err "release checksums do not contain exactly one ${artifact_name}"; exit 1; }
  case "$expected" in ''|*[!0-9a-fA-F]*) err "release checksum is malformed"; exit 1 ;; esac
  [ "$(printf '%s' "$expected" | awk '{ print length }')" -eq 64 ] || { err "release checksum is malformed"; exit 1; }
  if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$artifact" | awk '{ print $1 }')
  elif command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$artifact" | awk '{ print $1 }')
  else
    err "sha256sum or shasum is required to verify release artifacts"
    exit 1
  fi
  expected=$(printf '%s' "$expected" | tr '[:upper:]' '[:lower:]')
  actual=$(printf '%s' "$actual" | tr '[:upper:]' '[:lower:]')
  [ "$actual" = "$expected" ] || { err "checksum verification failed for ${artifact}"; exit 1; }
}

verify_artifact_identity() {
  artifact="$1"
  version="$2"
  chmod 700 "$artifact" || { err "verified release artifact could not be made private and executable"; exit 1; }
  output=$("$artifact" --version 2>&1) || { err "verified release artifact could not report its version"; exit 1; }
  expected="skilltap ${version#v}"
  [ "$output" = "$expected" ] || { err "release artifact identity does not match ${version}"; exit 1; }
}

bootstrap_binary_is_healthy() {
  # The Rust renderer emits a compact deterministic object with id then status.
  # Require exactly one successful binary resource; harness attention is then
  # safe to report separately, while binary attention remains fatal here.
  result="$1"
  count=$(printf '%s' "$result" | awk -F '"id":"binary"' '{ total += NF - 1 } END { print total + 0 }')
  [ "$count" -eq 1 ] || return 1
  status=$(printf '%s' "$result" | sed -n 's/.*"id":"binary","status":"\([^"]*\)".*/\1/p')
  case "$status" in installed|updated|no-op) return 0 ;; *) return 1 ;; esac
}

detect_platform() {
  OS=$(uname -s)
  ARCH=$(uname -m)
  case "$OS" in Linux*) OS=linux ;; Darwin*) OS=darwin ;; *) err "unsupported OS: $OS"; exit 1 ;; esac
  case "$ARCH" in x86_64|amd64) ARCH=x64 ;; aarch64|arm64) ARCH=arm64 ;; *) err "unsupported architecture: $ARCH"; exit 1 ;; esac
}

get_latest_version() {
  info "Fetching latest release..."
  response_file=$(mktemp) || { err "mktemp is required"; exit 1; }
  response_meta="${response_file}.meta"
  download "https://api.github.com/repos/${REPO}/releases/latest" "$response_file" "$MAX_METADATA_BYTES"
  RESPONSE=$(cat "$response_file")
  rm -f "$response_file" "$response_meta"
  VERSION=$(printf '%s' "$RESPONSE" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\(v[^"[:cntrl:]]*\)".*/\1/p')
  [ -n "$VERSION" ] && validate_release_tag "$VERSION" || { err "latest release metadata has no valid semver tag"; exit 1; }
}

main() {
  bold "skilltap installer"
  echo ""
  require_command awk
  require_command sed
  require_command uname
  require_command mktemp
  require_command dirname
  require_command id
  require_command stat
  validate_install_dir
  detect_platform
  get_latest_version

  ASSET="skilltap-${OS}-${ARCH}"
  URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
  CHECKSUMS_URL="https://github.com/${REPO}/releases/download/${VERSION}/checksums.txt"
  destination="${INSTALL_DIR}/${BINARY_NAME}"
  info "Downloading ${ASSET} (${VERSION})..."
  TMPFILE=$(mktemp)
  CHECKSUMS_FILE=$(mktemp)
  trap 'rm -f "$TMPFILE" "$TMPFILE.meta" "$CHECKSUMS_FILE" "$CHECKSUMS_FILE.meta"' EXIT HUP INT TERM
  download "$URL" "$TMPFILE"
  download "$CHECKSUMS_URL" "$CHECKSUMS_FILE" "$MAX_CHECKSUM_BYTES"
  verify_checksum "$CHECKSUMS_FILE" "$TMPFILE" "$ASSET"
  verify_artifact_identity "$TMPFILE" "$VERSION"
  ok "Verified ${ASSET} checksum and release identity"

  if [ -f "$destination" ] && [ ! -L "$destination" ]; then
    BOOTSTRAP_STATUS=0
    BOOTSTRAP_RESULT=$(SKILLTAP_INSTALL="$destination" "$destination" bootstrap --target all --json 2>&1) || BOOTSTRAP_STATUS=$?
    [ -n "$BOOTSTRAP_RESULT" ] && printf '%s\n' "$BOOTSTRAP_RESULT"
    if [ "$BOOTSTRAP_STATUS" -ne 0 ] && [ "$BOOTSTRAP_STATUS" -ne 2 ]; then
      err "existing skilltap bootstrap failed; the prior binary was preserved"
      exit "$BOOTSTRAP_STATUS"
    fi
    bootstrap_binary_is_healthy "$BOOTSTRAP_RESULT" || {
      err "existing skilltap bootstrap did not complete a verified binary operation"
      exit 1
    }
    ok "skilltap is already installed at ${destination}"
    exit 0
  fi

  # The verified candidate delegates publication and harness setup to the
  # shared Rust bootstrap boundary. The shell never overwrites the destination
  # and therefore preserves an existing binary on every bootstrap failure.
  BOOTSTRAP_STATUS=0
  BOOTSTRAP_RESULT=$(SKILLTAP_INSTALL="$destination" "$TMPFILE" bootstrap --target all --json 2>&1) || BOOTSTRAP_STATUS=$?
  [ -n "$BOOTSTRAP_RESULT" ] && printf '%s\n' "$BOOTSTRAP_RESULT"
  if [ "$BOOTSTRAP_STATUS" -ne 0 ] && [ "$BOOTSTRAP_STATUS" -ne 2 ]; then
    err "skilltap bootstrap failed before harness setup; existing binary was preserved"
    exit "$BOOTSTRAP_STATUS"
  fi
  bootstrap_binary_is_healthy "$BOOTSTRAP_RESULT" || {
    err "skilltap bootstrap did not complete the verified binary operation; existing binary was preserved"
    exit 1
  }
  [ -f "$destination" ] && [ ! -L "$destination" ] || { err "bootstrap did not publish a regular binary"; exit 1; }
  verify_artifact_identity "$destination" "$VERSION"
  ok "Installed skilltap ${VERSION} to ${destination}"
  echo ""

  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      bold "Add skilltap to your PATH:"
      echo ""
      echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
      echo ""
      ;;
  esac
}

main "$@"
