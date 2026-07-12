---
id: epic-safe-update-automation-diagnostics-status
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-diagnostics
depends_on: [epic-safe-update-automation-diagnostics-record]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Project Daemon Health in Status

Render service definition and typed last-run state through daemon status and
ordinary status in plain and JSON modes.

Acceptance criteria:

- Disabled, never-run, completed, pending, contended, and failed states differ.
- Manager absence remains safe and actionable.
- Rendering is read-only and secret-safe.

## Implementation Notes

- `daemon status` now reports service installation/manager reachability plus
  last-run result, timestamp, counts, and bounded failure code.
- Ordinary `status` also projects the optional daemon run record from state;
  both views remain derived from typed data and never render manager output.
- Verification: CLI/state tests and clippy passed.

## Review Record

- Inline review: **pass**. Disabled, never-run, installed, pending, contended,
  and failed states remain distinguishable without mutation.
