---
id: epic-safe-update-automation-service-definition
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-service
depends_on: []
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Render Deterministic User-Service Definitions

Generate bounded launchd and systemd-user definitions for one finite
`skilltap daemon run` cycle with no shell interpolation or secrets.

Acceptance criteria:

- Same spec produces byte-stable definitions.
- Invalid executable paths and intervals fail before rendering.
- Definitions contain one finite daemon invocation.

## Implementation Notes

- Added core deterministic launchd and systemd-user renderers with stable
  names, bounded intervals, absolute executable validation, and escaped paths.
- Both formats invoke exactly one `daemon run` cycle and contain no shell
  interpolation or secrets.
- Verification: targeted daemon definition tests and core clippy passed.

## Review Record

- Inline review: **pass**. Definitions are complete, deterministic, and
  platform-specific without introducing resident scheduling behavior.
