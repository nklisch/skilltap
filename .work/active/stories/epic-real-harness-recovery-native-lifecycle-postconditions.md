---
id: epic-real-harness-recovery-native-lifecycle-postconditions
kind: story
stage: review
tags: [correctness, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on:
  - epic-real-harness-recovery-native-lifecycle-contracts
  - epic-real-harness-recovery-native-lifecycle-managed-project
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Verify lifecycle postconditions with actionable diagnostics

## Scope

Replace generic/indeterminate observation handling with typed native evidence
and require a fresh target/scope postcondition before lifecycle success is
journaled. This story owns blocker 10.

## Acceptance

- Native list command failures, malformed JSON, unsupported shapes, ambiguous
  scope, and unmet presence expectations have distinct stable diagnostics and
  actionable next steps.
- A successful install/add/update is recorded only when the resource is freshly
  present in the requested target/scope; removal requires freshly missing.
- Prior journal success plus indeterminate observation is attention-required,
  never a false no-op or automatic duplicate mutation.
- Failed post-observation does not publish successful state and preserves a safe
  retry path without attempting an unverified rollback.
- Disposable fake and real harness coverage proves success, each failure class,
  and immediate repeat idempotence without touching user configuration.

## Implementation notes

- Execution capability: strongest inline implementation because the change guards native mutation and state publication across the harness/application boundary.
- Review weight: highest, from the caller's autopilot instruction.
- Files changed: `crates/harnesses/src/lifecycle.rs`, `crates/harnesses/src/lib.rs`, `crates/harnesses/src/bootstrap.rs`, `crates/harnesses/tests/lifecycle_scope.rs`, `crates/cli/src/application/lifecycle.rs`, `crates/cli/tests/native_postconditions.rs`, and `crates/test-support/src/native_process.rs`.
- Tests added: typed observation/postcondition unit coverage; isolated scoped-observation coverage; compiled CLI coverage for every failure class, failed-journal safety, prior-success indeterminate no-repeat behavior, install/remove postconditions, and immediate install/remove repeat idempotence.
- Discrepancies from design: the execution port uses the existing bounded per-operation process limit and a matching bounded JSON limit rather than adding a second constructor parameter; behavior and safety bounds remain the designed values.
- Adjacent issues parked: none.
- Verification: harness lifecycle unit and scope tests pass; the new compiled postcondition suite passes; 47 of 48 pre-existing compiled tests pass, with the sole stale assertion in `populated_plan_and_sync_apply_the_desired_inventory_resource` now needing to accept the truthful `repair` plan status after its lifecycle-aware fake reports the native resource still present.

## Review findings (2026-07-12)

- **Blocker — failed postcondition retries can repeat a completed native
  mutation.** The pre-mutation observation gate recognizes only an `Applied` or
  `NoChange` journal result. If the native mutation succeeds but its fresh list
  observation is indeterminate, skilltap journals `Failed`; once observation
  recovers, the next retry skips the precondition and runs the mutation again
  even when the exact resource is already present. An isolated compiled-binary
  reproduction observed two install invocations and `changed: true` on the
  retry. Fix and regression coverage are tracked by
  `epic-real-harness-recovery-native-lifecycle-postcondition-retry-safety`.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: unsafe retry after a failed/indeterminate postcondition
(`epic-real-harness-recovery-native-lifecycle-postcondition-retry-safety`)

**Important**: none

**Nits**: none

**Notes**: Fresh-context deep review at caller-selected `standard` weight,
escalated for native mutation and persisted-journal correctness. Typed failure,
expected-presence/removal, prior-success no-repeat, scope parsing, and isolated
plain/JSON tests were inspected. A disposable HOME/XDG/Claude-root reproduction
proved the retry mutation; no operator configuration was read or written.

## Repair notes (2026-07-12)

- Failed native first attempts now retain only native/harness-owned recovery evidence; managed projection seeds remain success-only.
- Every prior lifecycle attempt is freshly observed before retry. Desired state becomes a journaled no-op, opposite state permits one mutation, and indeterminate state remains mutation-free.
- Verified no-op evidence is re-observed by the native port under the configuration lock before `NoChange` is journaled, closing the planning-to-apply race.
- Isolated regressions cover install and removal recovery, opposite-state retry, mutation counts, journal transitions, and a deliberate revalidation race.
