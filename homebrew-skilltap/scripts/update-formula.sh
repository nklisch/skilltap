#!/bin/sh
# Update skilltap.rb formula with new version and checksums
# Usage: ./scripts/update-formula.sh <version> <checksums-file>
set -e

VERSION="${1#v}"
CHECKSUMS_FILE="$2"

if [ -z "$VERSION" ] || [ -z "$CHECKSUMS_FILE" ]; then
  echo "Usage: $0 <version> <checksums-file>"
  exit 1
fi

FORMULA="Formula/skilltap.rb"

# Extract checksums for each platform
get_checksum() {
  awk -v asset="$1" '$2 == asset || $2 == "*" asset { print $1 }' "$CHECKSUMS_FILE"
}

DARWIN_ARM64="$(get_checksum 'skilltap-darwin-arm64')"
DARWIN_X64="$(get_checksum 'skilltap-darwin-x64')"
LINUX_ARM64="$(get_checksum 'skilltap-linux-arm64')"
LINUX_X64="$(get_checksum 'skilltap-linux-x64')"

for CHECKSUM in "$DARWIN_ARM64" "$DARWIN_X64" "$LINUX_ARM64" "$LINUX_X64"; do
  case "$CHECKSUM" in
    ''|*[!0-9a-fA-F]* )
      echo "error: missing or invalid asset checksum in $CHECKSUMS_FILE" >&2
      exit 1
      ;;
  esac
  if [ "${#CHECKSUM}" -ne 64 ]; then
    echo "error: asset checksum must contain 64 hexadecimal characters" >&2
    exit 1
  fi
done

# Update version
sed -i "s/version \"[^\"]*\"/version \"${VERSION}\"/" "$FORMULA"

# Update each sha256 in order (darwin-arm64, darwin-x64, linux-arm64, linux-x64)
# Use awk to replace the nth sha256 occurrence
update_sha() {
  OCCURRENCE="$1"
  NEW_SHA="$2"
  awk -v n="$OCCURRENCE" -v sha="$NEW_SHA" '
    /sha256/ { count++; if (count == n) { sub(/sha256 "[^"]*"/, "sha256 \"" sha "\"") } }
    { print }
  ' "$FORMULA" > "${FORMULA}.tmp" && mv "${FORMULA}.tmp" "$FORMULA"
}

update_sha 1 "$DARWIN_ARM64"
update_sha 2 "$DARWIN_X64"
update_sha 3 "$LINUX_ARM64"
update_sha 4 "$LINUX_X64"

echo "Updated $FORMULA to version $VERSION"
