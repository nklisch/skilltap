---
id: story-fix-systemd-daemon-ownership
kind: story
stage: done
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Recognize Skilltap Systemd Timer Ownership

## Symptom

After daemon enable writes a service and timer, `daemon status` and `daemon
disable` classify the skilltap timer as an unmanaged conflict.

## Root cause

Systemd ownership detection required the `daemon run` command, which appears in
the service unit but not in the timer unit.

## Fix approach

Recognize the skilltap timer's exact service-unit reference alongside the
existing service marker, while continuing to reject unrelated timers.

## Regression test

`crates/cli/src/daemon.rs` tests service, timer, and unrelated systemd content.

## Implementation notes

- `crates/cli/src/daemon.rs` recognizes the exact skilltap timer unit reference
  in addition to the service command marker.
- Unrelated systemd timers remain conflicts.
- Full workspace tests and clippy with `-D warnings` pass.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fast-lane substrate review. The service/timer ownership regression
and green full workspace verification were confirmed; no lens walk was needed
for this story.
