---
id: story-skilltap-plugin-distribution-bootstrap-artifact-portable-rollback-safety
kind: story
stage: implementing
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Make artifact publication and rollback safe on every supported platform

Close the remaining artifact-boundary gaps found while reviewing
`story-skilltap-plugin-distribution-bootstrap-artifact-boundary-hardening`.
The Linux `renameat2` path is identity-safe, but the non-Linux fallbacks use
ordinary `fs::rename`, which can overwrite a destination that appears after
the initial observation. The no-prior rollback path also checks identity and
then unlinks by pathname, allowing a replacement race to remove an unrelated
file.

Acceptance criteria:

- macOS and Linux publication paths never overwrite a destination that appears
  or changes after observation; unsupported atomic primitives fail closed with
  `DestinationChanged`/`InstallFailed` rather than using overwrite-capable
  fallback behavior.
- Rollback with no prior payload removes only the expected published identity,
  using an atomic identity-safe operation; a replacement after observation is
  preserved.
- Existing exchange rollback behavior remains replacement-safe on both
  supported platforms, with cleanup on every failed/interrupted path.
- Isolated test-support coverage exercises the redirect-hop rejection,
  oversized/symlink payloads, checksum/permission/interruption cleanup,
  replacement races, and post-publish rollback preservation required by the
  bootstrap artifact contract. Tests remain offline and use temporary roots.

## Review origin

Standard fresh-context review of `614a29c`/`a1610a4` found an overwrite-capable
non-Linux fallback, a stat-then-unlink rollback race, and missing fixture
coverage for the full acceptance matrix.

## Implementation notes

- Execution capability: highest; this is a cross-platform security-sensitive
  filesystem boundary.
- Review weight: standard (autopilot caller policy).
- Adjacent issues parked: none.
