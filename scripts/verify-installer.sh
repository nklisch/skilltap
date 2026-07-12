#!/bin/sh
# Static and isolated installer contract checks. The fixture never contacts the
# network and redirects every write into a temporary HOME/install root.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
INSTALLER=$ROOT/install.sh

grep -q 'sha256sum' "$INSTALLER" || { echo "installer must verify sha256sum" >&2; exit 1; }
grep -q 'shasum -a 256' "$INSTALLER" || { echo "installer must support macOS shasum" >&2; exit 1; }
grep -q 'bootstrap --target all --json' "$INSTALLER" || { echo "installer must delegate bootstrap" >&2; exit 1; }
grep -q 'SKILLTAP_INSTALL=' "$INSTALLER" || { echo "installer must pass the verified destination" >&2; exit 1; }
grep -q 'validate_install_dir' "$INSTALLER" || { echo "installer must validate its destination" >&2; exit 1; }
grep -q -- '--max-filesize' "$INSTALLER" || { echo "installer downloads must be bounded" >&2; exit 1; }
grep -q -- '--max-redirs' "$INSTALLER" || { echo "installer redirects must be bounded" >&2; exit 1; }
if grep -Eq '(^|[[:space:];])mv([[:space:]]|$)|eval|sudo|sh -c|bash -c' "$INSTALLER"; then
  echo "installer contains an unsafe overwrite or shell execution path" >&2
  exit 1
fi

TMP_ROOT=$(mktemp -d)
trap 'rm -rf "$TMP_ROOT"' EXIT HUP INT TERM
FAKE_BIN="$TMP_ROOT/fake-bin"
FAKE_HOME="$TMP_ROOT/home"
INSTALL_ROOT="$TMP_ROOT/install"
mkdir -p "$FAKE_BIN" "$FAKE_HOME" "$INSTALL_ROOT"

cat >"$TMP_ROOT/artifact" <<'EOF'
#!/bin/sh
case "${1:-}" in
  --version) printf 'skilltap 3.0.0\n' ;;
  bootstrap)
    if [ "${FAKE_MODE:-}" = binary-attention ]; then
      printf '%s\n' '{"schema":1,"command":"bootstrap","result":"attention_required","summary":{},"resources":[{"id":"binary","status":"unknown-version"}],"operations":[],"warnings":[],"errors":[],"next_actions":[]}'
      exit 2
    fi
    cp "$0" "$SKILLTAP_INSTALL"
    chmod 700 "$SKILLTAP_INSTALL"
    printf '%s\n' '{"schema":1,"command":"bootstrap","result":"attention_required","summary":{},"resources":[{"id":"binary","status":"installed"},{"id":"claude","status":"unavailable"}],"operations":[],"warnings":[],"errors":[],"next_actions":[]}'
    exit 2
    ;;
  *) exit 1 ;;
esac
EOF
chmod 700 "$TMP_ROOT/artifact"
ARTIFACT_SHA=$(sha256sum "$TMP_ROOT/artifact" | awk '{ print $1 }')

cat >"$FAKE_BIN/curl" <<'EOF'
#!/bin/sh
set -eu
output=
url=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output) output=$2; shift 2 ;;
    --write-out) shift 2 ;;
    *) url=$1; shift ;;
  esac
done
[ -n "$output" ] || exit 1
case "$url" in
  */releases/latest)
    if [ "${FAKE_MODE:-}" = hostile-redirect ]; then
      printf '%s\n%s\n' 302 'https://evil.example/latest'
      exit 0
    fi
    if [ "${FAKE_MODE:-}" = malformed-metadata ]; then
      printf '%s' '{"tag_name":"latest"}' >"$output"
    else
      printf '%s' '{"tag_name":"v3.0.0"}' >"$output"
    fi
    ;;
  */checksums.txt)
    if [ "${FAKE_MODE:-}" = checksum-failure ]; then
      printf '%s  skilltap-linux-x64\n' '0000000000000000000000000000000000000000000000000000000000000000' >"$output"
    else
      printf '%s  skilltap-linux-x64\n' "$ARTIFACT_SHA" >"$output"
    fi
    ;;
  *) cp "$FAKE_ROOT/artifact" "$output" ;;
esac
printf '200\n%s\n' "$url"
EOF
chmod 700 "$FAKE_BIN/curl"

run_fixture() {
  mode=${1:-healthy}
  ARTIFACT_SHA="$ARTIFACT_SHA" HOME="$FAKE_HOME" PATH="$FAKE_BIN:$PATH" FAKE_ROOT="$TMP_ROOT" FAKE_MODE="$mode" \
    SKILLTAP_INSTALL="$INSTALL_ROOT" sh "$INSTALLER" >/dev/null
}

run_fixture healthy
[ -f "$INSTALL_ROOT/skilltap" ] && [ ! -L "$INSTALL_ROOT/skilltap" ] || { echo "isolated install did not publish a regular binary" >&2; exit 1; }
run_fixture healthy

if run_fixture malformed-metadata 2>/dev/null; then
  echo "malformed release metadata was accepted" >&2
  exit 1
fi
if run_fixture checksum-failure 2>/dev/null; then
  echo "checksum failure was accepted" >&2
  exit 1
fi
if run_fixture hostile-redirect 2>/dev/null; then
  echo "hostile redirect was accepted" >&2
  exit 1
fi

ln -s "$TMP_ROOT/real-destination" "$TMP_ROOT/unsafe-link"
if HOME="$FAKE_HOME" PATH="$FAKE_BIN:$PATH" FAKE_ROOT="$TMP_ROOT" SKILLTAP_INSTALL="$TMP_ROOT/unsafe-link" sh "$INSTALLER" >/dev/null 2>&1; then
  echo "symlink destination was accepted" >&2
  exit 1
fi

cp "$INSTALL_ROOT/skilltap" "$TMP_ROOT/prior"
if run_fixture binary-attention 2>/dev/null; then
  echo "binary attention was accepted as installer success" >&2
  exit 1
fi
cmp "$INSTALL_ROOT/skilltap" "$TMP_ROOT/prior" || { echo "binary attention replaced the prior binary" >&2; exit 1; }

echo "installer contract checks passed"
