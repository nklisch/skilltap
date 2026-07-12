---
id: story-skilltap-plugin-distribution-bootstrap-installer
kind: story
stage: implementing
tags: [infra, content, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-command]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Online installer and plugin bootstrap parity

Align the curl/wget installer and the self-hosted plugin's agent guidance with
the first-class bootstrap command. After checksum verification and binary
installation, the online installer invokes the same Rust bootstrap boundary;
it detects Claude/Codex read-only and reports binary availability separately
from plugin setup. No second shell implementation of harness mutation is
allowed.

Scope:

- `install.sh` and a static `scripts/verify-installer.sh` check.
- Website getting-started/updates installation guidance and generated copies
  where applicable.
- Compiled/isolated installer contract tests.

Acceptance criteria:

- Clean isolated Linux/macOS fixtures can install a verified binary, detect
  fake harness executables, and invoke the shared bootstrap boundary without
  touching the operator's HOME, active sibling `../skills`, or native caches.
- Installer and binary choose the same latest asset/checksum protocol and are
  idempotent on a healthy repeat; unsupported platforms, malformed metadata,
  checksum failure, missing dependencies, and unsafe destinations fail safely.
- Website presents marketplace plugin installation and one-line installation
  as equal first-class paths, links to executable `skilltap bootstrap --help`,
  and does not reproduce a second command grammar.
- Missing or unsupported harness setup appears as a separate actionable result,
  never a false complete binary status. No arbitrary post-install command,
  shell interpolation, root escalation, or cache write is introduced.

If a documented native hook is later available, package it only through the
attested channel contract; otherwise preserve the explicit agent-invocable
bootstrap path and record the limitation in the plugin guidance.

## Implementation notes
- Execution capability: highest available local capability; installer parity and plugin handoff affect security and release trust.
- Review weight: standard (source: autopilot project default).
- Files changed: `install.sh`, `scripts/verify-installer.sh`, `website/guide/getting-started.md`, `website/guide/updates.md`, compiled bootstrap coverage.
- Tests added: POSIX syntax/static installer contract checks and isolated compiled binary install/no-op/major acknowledgment scenarios.
- Discrepancies from design: the shell entrypoint delegates post-install harness setup to the verified binary and accepts attention exit 2 while failing closed on pre-mutation errors.
- Adjacent issues parked: none.
