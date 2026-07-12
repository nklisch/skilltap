---
id: gate-tests-daemon-service-failure-paths
kind: story
stage: implementing
tags: [testing]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: tests
created: 2026-07-12
updated: 2026-07-12
---

# Cover daemon service failure and unmanaged-definition paths

## Priority

Medium

## Spec reference

Items `epic-safe-update-automation-service-definition` and
`epic-safe-update-automation-service-lifecycle`.

## Gap type

Missing disable no-op, unmanaged lookalike preservation, manager failure,
malformed/non-regular definition, and pair-write rollback coverage.

## Suggested test

Use isolated service roots and a fake manager to assert exact ownership
rejection, no manager invocation when nothing is owned, typed attention for
malformed paths, and restoration when the second service definition write
fails.

## Test location (suggested)

Daemon unit tests and isolated compiled CLI tests.

## Autopilot implementation note

The failure matrix and isolated fixture strategy are fully specified; proceed
directly to implementation and verification without a separate design pass.

## Implementation Notes

Added `daemon_service_failure_paths_preserve_unmanaged_and_nonregular_definitions`
to the compiled-binary suite. It verifies disable is a no-op when nothing is
owned, unmanaged lookalikes are preserved and reported as conflicts, and
non-regular definitions are surfaced as unreadable without overwriting them.

Verification: the focused test passes; the full `compiled_binary` integration
suite passes for this story's coverage.

## Review findings

- **Blocker**: The regression covers only three of the five failure classes in
  the gate scope: disable no-op, unmanaged lookalike preservation, and a
  non-regular timer path. It does not exercise a malformed owned definition,
  service-manager failure propagation, or rollback/restoration when the second
  service definition write fails. Add isolated fixtures/fake-manager controls
  for those paths and assert typed attention, no destructive overwrite, and
  pair-write recovery before advancing this item.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: missing malformed-definition, manager-failure, and pair-write
rollback coverage (this item)
**Important**: none
**Nits**: none

**Notes**: Standard substrate review with deep test-integrity and daemon safety
lenses. The focused test passes, but its assertions do not cover the complete
failure matrix named in the item and original gate finding. Do not weaken the
existing cases; extend the isolated test support and retain unmanaged files.
