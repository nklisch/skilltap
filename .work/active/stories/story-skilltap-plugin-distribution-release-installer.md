---
id: story-skilltap-plugin-distribution-release-installer
kind: story
stage: implementing
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-release
depends_on: [story-skilltap-plugin-distribution-release-contract]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Align one-line installation with bootstrap

Make `install.sh` delegate verified binary publication and harness resource
setup to the Rust bootstrap boundary. Detect Claude/Codex executables without
enabling them, preserve a healthy prior binary on failures, and report binary
and harness outcomes independently.

Acceptance criteria:

- Shell and Rust use the same bounded redirect, checksum, platform, and release
  identity rules.
- Installer reruns are idempotent and never write native caches or imply
  `harness enable`.
- Offline shell fixtures cover malformed metadata, redirect/permission/
  checksum failures, cleanup, and mixed harness attention.

## Implementation notes
- Execution capability: highest; installer is a binary supply-chain boundary.
- Review weight: standard (autopilot caller policy).
