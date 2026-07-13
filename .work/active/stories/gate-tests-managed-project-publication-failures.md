---
id: gate-tests-managed-project-publication-failures
kind: story
stage: review
tags: [testing]
parent: null
depends_on: []
release_binding: 3.0.2
gate_origin: tests
created: 2026-07-12
updated: 2026-07-12
---

# Exercise every managed project publication failure boundary

## Priority
Critical

## Spec reference
`epic-real-harness-recovery-native-lifecycle-managed-project-load-contract`:
tree, catalog, config, and state failures restore prior state or report exact
owned residuals.

## Required test
Inject each failure, verify restoration or exact residual output, then verify a
single successful retry followed by a no-op.

## Implementation

- Added test-only dependency injection for isolated platform paths and the
  managed project filesystem while preserving `SystemFileSystem` as the
  production default.
- Exercised catalog, complete skill tree, Codex config, and pending-state
  publication failures through `execute_native_lifecycle`.
- Verified every injected failure restores the absent prior surfaces, one
  retry publishes the desired projection, and an immediate repeat is a no-op
  without another tree publication.

## Verification

- `cargo test -p skilltap --lib managed_project_publication_failures_restore_then_retry_once_and_noop`
