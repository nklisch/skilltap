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

## Review findings

- **Blocker — the injected state boundary does not verify state restoration.**
  The `Boundary::State` path fails the first state write, but the assertions at
  `crates/cli/src/application/tests.rs:547-570` check only that project skill
  and MCP surfaces are absent. They never snapshot the state document before
  the operation or require byte/domain equality afterward, so the test would
  not catch a stale Pending/Applied record or another state mutation after the
  injected failure. Add that exact before/after assertion, then retain the
  successful retry and immediate no-op checks.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: state-boundary restoration is not asserted
**Important**: none
**Nits**: the recorded focused command uses `--exact` without the module-qualified test name and therefore selects zero tests; the actual passing command is `cargo test -p skilltap --lib application::tests::managed_project_publication_failures_restore_then_retry_once_and_noop -- --exact`
**Rejected**: none

**Notes**: Substrate review at effective `standard` weight (caller-selected),
escalated to the Deep lane because this is a critical persistence/rollback
story. Same-harness fresh-context review inspected commit `c5fa054`, the real
application lifecycle service, executor/journal ordering, confined filesystem
adapter, rollback implementation, and the fault-injection test. The injected
catalog/tree/config/state faults do traverse `execute_native_lifecycle`; the
filesystem adapter delegates all non-fault operations to the production
implementation, and the rollback residual reporting code re-observes restored
surfaces rather than assuming success. The correctly qualified focused test
passes. Security was limited to the changed filesystem/state seams; no public
CLI, schema, or foundation-doc change was introduced by this story.

## Review resolution

- The state-boundary case now snapshots `state.json` immediately before the
  injected pending write failure and requires exact byte equality afterward.
- The existing retry still performs one real publication and its immediate
  repeat remains a no-op.
