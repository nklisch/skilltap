---
id: story-fix-daemon-service-safety
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

# Harden daemon service ownership and publication

## Symptom

Lookalike service files can be treated as skilltap-owned, malformed daemon
state is masked as a healthy disabled daemon, repeated enable rewrites files,
and a failed second write can leave a partial service pair.

## Root cause

Ownership relies on broad substring checks, status suppresses state and file
read errors, and enable publishes each generated file independently without
byte comparison or rollback.

## Fix approach

Add an exact generated marker and stricter shape checks, surface malformed or
unreadable state/definitions as attention, publish only changed files with
rollback, and make disable a no-op when no owned definition exists.

## Regression test

Daemon unit and compiled-binary tests cover lookalikes, malformed definitions,
repeat enable idempotence, and no-op disable behavior.

## Implementation notes

- Generated launchd/systemd files carry an explicit v3 ownership marker; systemd
  specifier percent escaping is covered by a core test.
- Ownership checks reject lookalikes, daemon enable compares bytes and rolls back
  prior writes on failure, and disable skips manager calls when nothing owned is
  installed.
- Status now surfaces malformed state and unreadable definitions as attention.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fast-lane substrate review. Implementation notes and green daemon,
compiled-binary, workspace, and clippy verification were confirmed; no lens
walk was needed for this story.
