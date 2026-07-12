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
