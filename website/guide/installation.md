---
description: Install skilltap via curl, Homebrew, bunx, or direct binary download. Available on Linux and macOS (x64 and ARM64). No runtime dependencies required.
---

# Installation

## Recommended: curl installer

The fastest way to install skilltap:

```bash
curl -fsSL https://skilltap.dev/install.sh | sh
```

This downloads the latest binary for your platform and installs it to `~/.local/bin/skilltap`.

To install to a different location, set `SKILLTAP_INSTALL`:

```bash
SKILLTAP_INSTALL=/usr/local/bin curl -fsSL https://skilltap.dev/install.sh | sh
```

## Via Homebrew

On macOS and Linux with [Homebrew](https://brew.sh):

```bash
brew install skilltap/skilltap/skilltap
```

This installs a pre-built binary and keeps it up to date with `brew upgrade`.

## Via bunx

If you have [Bun](https://bun.sh) installed, you can run skilltap without installing it:

```bash
bunx skilltap --help
```

## Direct binary download

Pre-built binaries are available on the [GitHub Releases](https://github.com/nklisch/skilltap/releases) page for every supported platform:

| Platform     | Architecture |
| ------------ | ------------ |
| Linux        | x64          |
| Linux        | ARM64        |
| macOS        | x64          |
| macOS        | ARM64        |

Download the binary for your platform, make it executable, and move it to a directory on your PATH.

Each release includes a `checksums.txt` file with SHA-256 hashes for all binaries. To verify your download:

```bash
sha256sum -c checksums.txt --ignore-missing
```

## Platform support

skilltap supports **Linux** and **macOS** on both x64 and ARM64 architectures.

Windows is not yet supported.

## Verify installation

After installing, confirm everything is working:

```bash
skilltap --help
```

You should see the list of available commands and global flags.

## Shell completions (optional)

Set up tab-completion for your shell:

```bash
skilltap completions bash --install   # bash
skilltap completions zsh --install    # zsh
skilltap completions fish --install   # fish
```

See [Shell Completions](/guide/shell-completions) for details and troubleshooting.
