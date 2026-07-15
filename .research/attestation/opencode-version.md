---
source_handle: opencode-version
fetched: 2026-07-14
source_urls:
  - https://github.com/anomalyco/opencode/releases/tag/v1.18.1
  - https://registry.npmjs.org/opencode-ai/1.18.1
  - https://registry.npmjs.org/opencode-linux-x64/1.18.1
provenance: source-direct-plus-isolated-execution
substrate_confidence: source-direct-and-runtime
---

# OpenCode version profile

The current official release observed during the isolated validation run is
`v1.18.1`, published at the official `anomalyco/opencode` GitHub release. The
official npm launcher package is `opencode-ai@1.18.1`; the Linux x64 executable
was acquired from the official optional package `opencode-linux-x64@1.18.1`.

## Provenance

- Launcher tarball: `https://registry.npmjs.org/opencode-ai/-/opencode-ai-1.18.1.tgz`
- Launcher npm integrity: `sha512-Rtp0fCJyu3Iz0MXfwQeAYdYjIsSPPUWYyJO0mf0Q9v5zTNYxlakzXUh+Van50XAmEDAhCaJvCcOJzweq2k3HMQ==`
- Launcher npm SHA-1: `10be379c469487e0cb3c81f606ff54c16c7fb960`
- Linux x64 tarball: `https://registry.npmjs.org/opencode-linux-x64/-/opencode-linux-x64-1.18.1.tgz`
- Linux x64 npm integrity: `sha512-6OqBNhQHlJejNBdT5OBrXyLwAWRwc3zmEMYKKavWu3M7RpcuO044YdrW68ZsieU/r9hWZc8Jb1sNHub4kGdDDw==`
- Linux x64 tarball SHA-512: `e8ea813614079497a3341753e4e06b5f22f0016470737ce610c60a29abd6bb733b46972e3b4e3861dad6ebc66c89e53fafd85665cf096f5b0d1ee6f89067430f`
- Linux x64 tarball SHA-256: `694e4c7df5004d28d69dd79a81f6092ea300fb3c12ab4aca04a20985ee9be8ab`
- Linux x64 npm SHA-1: `6568fd2d509962b1ad6c0989af5056c5ff5bf7f5`
- npm registry signature key id: `SHA256:DhQ8wR5APBvFHLF/+Tc+AYvPOdTpcIDqOhxsBHRwC7U`

The SHA-512 digest of the acquired tarball decodes to the registry's published
integrity value. The launcher package's postinstall selects the platform
package and verifies that its binary responds successfully to `--version`.

## Isolated runtime evidence

The Linux x64 binary was run with isolated `HOME`, `XDG_CONFIG_HOME`,
`XDG_DATA_HOME`, `XDG_CACHE_HOME`, and a temporary Git project. Exact version
stdout was the seven-byte sequence `31 2e 31 38 2e 31 0a`, or `1.18.1\n`.

The runtime confirmed that project configuration merges with global
configuration, preserving non-conflicting values and overriding same-name MCP
servers. A project `same` server replaced the global `same` server in effective
`mcp list` output while the global declaration remained on disk.

The native process also creates its own data/model/package cache while running;
that is OpenCode runtime behavior, not a skilltap lifecycle surface. skilltap
must not write or reconcile `~/.cache/opencode`, Bun package directories, the
OpenCode database, or OAuth data.
