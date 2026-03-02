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

## Via bunx

If you have [Bun](https://bun.sh) installed, you can run skilltap without installing it:

```bash
bunx skilltap --help
```

## Via npx

If you have npm and Bun on your PATH:

```bash
npx skilltap --help
```

## Direct binary download

Pre-built binaries are available on the [GitHub Releases](https://github.com/skilltap/skilltap/releases) page for every supported platform:

| Platform     | Architecture |
| ------------ | ------------ |
| Linux        | x64          |
| Linux        | ARM64        |
| macOS        | x64          |
| macOS        | ARM64        |

Download the binary for your platform, make it executable, and move it to a directory on your PATH.

## Platform support

skilltap supports **Linux** and **macOS** on both x64 and ARM64 architectures.

Windows is not yet supported.

## Verify installation

After installing, confirm everything is working:

```bash
skilltap --help
```

You should see the list of available commands and global flags.
