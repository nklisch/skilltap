---
id: epic-safe-update-automation-service-run
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-service
depends_on: [epic-safe-update-automation-service-lifecycle]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Run One Safe Daemon Cycle

Invoke the same foreground safe-update planning/application service once per
daemon run, with no acknowledgments and bounded lock/resolver behavior.

Acceptance criteria:

- Daemon cycles never apply partial, drifted, pinned, or conflicted updates.
- Lock/source failures terminate with typed status and no hang.
- Repeating a cycle is idempotent and preserves unmanaged/drifted resources.

## Implementation Notes

- `daemon run` now performs one bounded `apply-safe` cycle: tracked native
  plugins and Git-backed standalone skills are delegated to their existing
  lifecycle executors; disabled/pinned/local/unresolved resources are skipped
  and remain pending.
- No acknowledgment selectors are supplied. Each child operation retains the
  existing lock, revalidation, idempotence, drift, and state-recording guards.
- Non-apply-safe policy exits read-only with an explicit warning.
- Verification: CLI daemon command coverage and clippy passed.

## Review Record

- Inline review: **pass**. The cycle is finite, non-interactive, and reuses
  native-first lifecycle paths without a daemon-only mutation bypass.
