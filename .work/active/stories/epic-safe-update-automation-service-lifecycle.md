---
id: epic-safe-update-automation-service-lifecycle
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-service
depends_on: [epic-safe-update-automation-service-definition]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Manage User-Service Lifecycle

Implement non-interactive enable, disable, and status operations with atomic
owned-definition publication and direct service-manager argv.

Acceptance criteria:

- Enable/disable are idempotent and preserve unmanaged service files.
- Manager failures are typed and do not remove valid owned definitions.
- Status is read-only and reports definition/manager state.

## Implementation Notes

- Added non-interactive daemon enable/disable/status dispatches. Definitions
  are written atomically under owned launchd/systemd-user paths and unmanaged
  conflicting files are preserved.
- Service-manager activation uses bounded direct argv through the existing
  executable/process runtime; manager failure retains a valid owned definition
  and reports attention.
- Updated compiled-binary coverage for the now-implemented lifecycle commands.
- Verification: CLI tests and clippy passed.

## Review Record

- Inline review: **pass**. Ownership checks precede writes/removals and no shell
  command strings or secrets cross the manager boundary.
