# Design: Distribution

skilltap already compiles to a standalone binary via `bun build --compile`. This doc covers the remaining distribution channels: Homebrew formula, curl install script, GitHub Releases CI, and Linux package considerations.

## Distribution Channels

| Channel | Audience | Status |
|---|---|---|
| `bunx skilltap` | Bun users | Works today |
| `npx skilltap` | Node users | Works today |
| Standalone binary | No runtime needed | Built today, not distributed |
| **Homebrew formula** | macOS + Linux Homebrew users | **New** |
| **curl install script** | Quick installs, CI | **New** |
| **GitHub Releases** | Direct download, CI artifacts | **New** |

## GitHub Releases CI

All other channels depend on this. A GitHub Actions workflow that builds binaries on every release tag and uploads them as release assets.

### Build Matrix

| Target | OS | Arch | Binary Name |
|---|---|---|---|
| `linux-x64` | ubuntu-latest | x86_64 | `skilltap-linux-x64` |
| `linux-arm64` | ubuntu-latest (arm64) | aarch64 | `skilltap-linux-arm64` |
| `darwin-x64` | macos-13 | x86_64 | `skilltap-darwin-x64` |
| `darwin-arm64` | macos-14 | arm64 | `skilltap-darwin-arm64` |

### Workflow

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ["v*"]

permissions:
  contents: write
  id-token: write
  attestations: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: linux-x64
            bun-target: bun-linux-x64
          - os: ubuntu-24.04-arm
            target: linux-arm64
            bun-target: bun-linux-arm64
          - os: macos-13
            target: darwin-x64
            bun-target: bun-darwin-x64
          - os: macos-14
            target: darwin-arm64
            bun-target: bun-darwin-arm64

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2

      - run: bun install
      - run: bun test

      - name: Build binary
        run: |
          bun build packages/cli/src/index.ts \
            --compile \
            --target=${{ matrix.bun-target }} \
            --outfile=skilltap-${{ matrix.target }}

      - name: Attest binary provenance
        uses: actions/attest-build-provenance@v2
        with:
          subject-path: skilltap-${{ matrix.target }}

      - uses: actions/upload-artifact@v4
        with:
          name: skilltap-${{ matrix.target }}
          path: skilltap-${{ matrix.target }}

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          merge-multiple: true

      - name: Generate checksums
        run: sha256sum skilltap-* > checksums.txt

      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            skilltap-*
            checksums.txt
          generate_release_notes: true

  publish-npm:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          registry-url: https://registry.npmjs.org
      - uses: oven-sh/setup-bun@v2
      - run: bun install
      - run: |
          cd packages/cli && npm publish --provenance --access public
          cd ../core && npm publish --provenance --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

### Release Assets

A release for `v0.2.0` would contain:

```
skilltap-linux-x64
skilltap-linux-arm64
skilltap-darwin-x64
skilltap-darwin-arm64
checksums.txt
```

Each binary is attested with GitHub's build provenance (Sigstore). Users can verify with:

```bash
gh attestation verify skilltap-linux-x64 --repo skilltap/skilltap
```

## Homebrew Formula

A formula in a tap repo (`homebrew-skilltap`) that downloads the pre-built binary from GitHub Releases.

### Tap Setup

```
skilltap/homebrew-skilltap/
  Formula/
    skilltap.rb
```

Users install with:

```bash
brew tap skilltap/skilltap
brew install skilltap
```

Or one-liner:

```bash
brew install skilltap/skilltap/skilltap
```

### Formula

```ruby
# Formula/skilltap.rb
class Skilltap < Formula
  desc "CLI for installing agent skills from any git host"
  homepage "https://github.com/skilltap/skilltap"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/skilltap/skilltap/releases/download/v0.2.0/skilltap-darwin-arm64"
      sha256 "abc123..."
    end
    on_intel do
      url "https://github.com/skilltap/skilltap/releases/download/v0.2.0/skilltap-darwin-x64"
      sha256 "def456..."
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/skilltap/skilltap/releases/download/v0.2.0/skilltap-linux-arm64"
      sha256 "789abc..."
    end
    on_intel do
      url "https://github.com/skilltap/skilltap/releases/download/v0.2.0/skilltap-linux-x64"
      sha256 "012def..."
    end
  end

  def install
    binary = Dir["skilltap-*"].first
    bin.install binary => "skilltap"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/skilltap --version")
  end
end
```

### Formula Auto-Update

A GitHub Actions workflow in the `homebrew-skilltap` repo that auto-bumps the formula when a new release is published on the main repo:

```yaml
# .github/workflows/update-formula.yml
name: Update Formula
on:
  repository_dispatch:
    types: [release]
  workflow_dispatch:
    inputs:
      version:
        required: true

jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download checksums
        run: |
          VERSION=${{ github.event.client_payload.version || inputs.version }}
          curl -sL "https://github.com/skilltap/skilltap/releases/download/v${VERSION}/checksums.txt" -o checksums.txt

      - name: Update formula
        run: |
          VERSION=${{ github.event.client_payload.version || inputs.version }}
          # Script that updates version and sha256 values in skilltap.rb
          ./scripts/update-formula.sh "$VERSION" checksums.txt

      - name: Create PR
        uses: peter-evans/create-pull-request@v6
        with:
          title: "skilltap ${{ github.event.client_payload.version || inputs.version }}"
          branch: "update-${{ github.event.client_payload.version || inputs.version }}"
```

The main repo's release workflow triggers this via `repository_dispatch`:

```yaml
# Added to .github/workflows/release.yml, after the release job
notify-homebrew:
  needs: release
  runs-on: ubuntu-latest
  steps:
    - uses: peter-evans/repository-dispatch@v3
      with:
        token: ${{ secrets.HOMEBREW_TAP_TOKEN }}
        repository: skilltap/homebrew-skilltap
        event-type: release
        client-payload: '{"version": "${{ github.ref_name }}"}'
```

## Install Script

A curl-pipe-sh script for quick installs. Downloads the right binary for the platform, puts it in `~/.local/bin/` or `/usr/local/bin/`.

### Usage

```bash
curl -fsSL https://skilltap.dev/install.sh | sh
```

Or with options:

```bash
curl -fsSL https://skilltap.dev/install.sh | sh -s -- --dir ~/.local/bin
curl -fsSL https://skilltap.dev/install.sh | sh -s -- --version 0.2.0
```

### Script

```bash
#!/bin/sh
set -euo pipefail

REPO="skilltap/skilltap"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
VERSION=""

# Parse args
while [ $# -gt 0 ]; do
  case "$1" in
    --dir) INSTALL_DIR="$2"; shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64) ARCH="x64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

case "$OS" in
  linux|darwin) ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

BINARY="skilltap-${OS}-${ARCH}"

# Resolve version
if [ -z "$VERSION" ]; then
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
fi

URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY}"
CHECKSUM_URL="https://github.com/${REPO}/releases/download/${VERSION}/checksums.txt"

echo "Installing skilltap ${VERSION} (${OS}/${ARCH})..."

# Download
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
curl -fsSL "$URL" -o "${TMPDIR}/skilltap"
curl -fsSL "$CHECKSUM_URL" -o "${TMPDIR}/checksums.txt"

# Verify checksum
EXPECTED=$(grep "$BINARY" "${TMPDIR}/checksums.txt" | awk '{print $1}')
ACTUAL=$(sha256sum "${TMPDIR}/skilltap" | awk '{print $1}')
if [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "Checksum verification failed!"
  echo "  Expected: $EXPECTED"
  echo "  Got:      $ACTUAL"
  exit 1
fi

# Install
chmod +x "${TMPDIR}/skilltap"
mkdir -p "$INSTALL_DIR"

if [ -w "$INSTALL_DIR" ]; then
  mv "${TMPDIR}/skilltap" "${INSTALL_DIR}/skilltap"
else
  sudo mv "${TMPDIR}/skilltap" "${INSTALL_DIR}/skilltap"
fi

echo "✓ Installed skilltap ${VERSION} to ${INSTALL_DIR}/skilltap"

# Check PATH
case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *) echo "  Note: ${INSTALL_DIR} is not in your PATH. Add it:";
     echo "    export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
esac
```

### Hosting

The install script lives at two locations:

1. **In the repo**: `scripts/install.sh` (source of truth)
2. **At a URL**: `https://skilltap.dev/install.sh` (redirect to raw GitHub, or hosted on a static site)

If no custom domain, use the raw GitHub URL:

```bash
curl -fsSL https://raw.githubusercontent.com/skilltap/skilltap/main/scripts/install.sh | sh
```

## Linux Packages

Not built in CI initially — too many formats and repos to maintain. Users on Linux use Homebrew (Linuxbrew), the install script, or npm/bunx. If demand materializes, add:

| Format | Tool | Audience |
|---|---|---|
| `.deb` | `dpkg` | Debian, Ubuntu |
| `.rpm` | `rpm` | Fedora, RHEL |
| AUR | `makepkg` | Arch Linux |
| Nix | `nix-env` | NixOS, Nix users |

These are deferred until there's user demand. The install script and Homebrew cover the vast majority of Linux users who would install a developer CLI tool.

## Versioning

All distribution channels use the same version. A single git tag (`v0.2.0`) triggers:

1. GitHub Release (binaries + checksums)
2. npm publish (`skilltap` CLI package + `@skilltap/core`)
3. Homebrew formula PR (auto-triggered from release)

The install script defaults to `latest` but supports `--version` for pinning.

## New Files

```
.github/workflows/release.yml          # Build + release + npm publish
scripts/install.sh                      # Curl install script
```

Plus the separate `homebrew-skilltap` repo:

```
skilltap/homebrew-skilltap/
  Formula/skilltap.rb
  scripts/update-formula.sh
  .github/workflows/update-formula.yml
```

## Testing

- **CI test**: release workflow runs on a test tag in a fork (verify all 4 binaries build)
- **Script test**: run `install.sh` in a clean Docker container (ubuntu, alpine), verify binary works
- **Homebrew test**: `brew install --build-from-source` against the formula (Homebrew's standard test)
- **Checksum test**: verify checksums.txt matches downloaded binaries
