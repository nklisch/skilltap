#!/bin/sh
# Validate release identity and canonical plugin publication inputs offline.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

VERSION=$(awk -F '"' '/^version = "/ { print $2; exit }' Cargo.toml)
[ -n "$VERSION" ] || { echo "error: workspace version is missing" >&2; exit 1; }

for manifest in plugin/.claude-plugin/plugin.json plugin/.codex-plugin/plugin.json; do
  grep -Fq '"name": "skilltap"' "$manifest" || {
    echo "error: $manifest has the wrong plugin identity" >&2
    exit 1
  }
  grep -Fq "\"version\": \"$VERSION\"" "$manifest" || {
    echo "error: $manifest version does not match $VERSION" >&2
    exit 1
  }
done

for catalog in plugin/.claude-plugin/marketplace.json plugin/.agents/plugins/marketplace.json; do
  grep -Fq '"name": "skilltap"' "$catalog" || {
    echo "error: $catalog has no skilltap entry" >&2
    exit 1
  }
  grep -Fq "\"version\": \"$VERSION\"" "$catalog" || {
    echo "error: $catalog version does not match $VERSION" >&2
    exit 1
  }
done

for asset in skilltap-linux-x64 skilltap-linux-arm64 skilltap-darwin-x64 skilltap-darwin-arm64; do
  grep -Fq "$asset" .github/workflows/release.yml || {
    echo "error: release workflow is missing $asset" >&2
    exit 1
  }
done

grep -Fq 'checksums.txt' .github/workflows/release.yml || {
  echo "error: release workflow does not publish checksums.txt" >&2
  exit 1
}
grep -Fq 'actions/attest-build-provenance' .github/workflows/release.yml || {
  echo "error: release workflow does not attest binary provenance" >&2
  exit 1
}

echo "release contract inputs verified for v$VERSION"
