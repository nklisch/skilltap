---
id: feature-daemon-marketplace-refresh-execution
kind: story
stage: done
tags: [infra]
parent: feature-daemon-marketplace-refresh
depends_on: [feature-daemon-marketplace-refresh-task-graph]
release_binding: 3.1.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Execute Marketplace Refresh and Plugin Updates as One Plan

## Checkpoint

Extract reusable exact-target native lifecycle planning and the existing hybrid
execution tail, then compose each daemon cycle's native marketplace refreshes
and tracked plugin updates into one dependency-aware plan executed under one
configuration lock. Foreground lifecycle behavior must remain unchanged.

Current-cycle daemon attempts must not be suppressed by prior successful journal
entries. A refresh failure or indeterminate postcondition must be journaled and
skip only its dependent plugin operations; unrelated branches continue. The
daemon never supplies acknowledgment. Add conservative revision-aware update
classification: only equal validated before/after revisions are `NoChange`;
changed or unavailable comparison evidence remains `Applied` after the existing
presence postcondition succeeds.

Persist typed operation references in the daemon run record and resolve status
from the ordinary target-local operation journal rather than copying native
errors or payloads into another result store.

## Expected implementation surface

- `crates/cli/src/application.rs`
- `crates/cli/src/application/lifecycle.rs`
- `crates/cli/src/application/execution.rs`
- `crates/core/src/lifecycle_operation.rs`
- `crates/harnesses/src/lifecycle.rs`
- `crates/harnesses/tests/lifecycle_scope.rs`
- `crates/core/src/storage/state.rs`
- `crates/cli/src/application/status.rs`

## Acceptance evidence

- Foreground marketplace/plugin lifecycle retains existing scope, target,
  acknowledgment, recovery, and managed-fallback behavior after extraction.
- One validated plan and lock cover refresh and plugin-update operations, with
  exact dependency edges and target-local journal updates.
- Failed refreshes suppress dependent plugin invocation while independent
  marketplaces continue.
- Equal validated plugin revisions produce `NoChange`; missing/malformed
  revision evidence never claims a false no-op.
- Daemon state stores only validated operation references, and status resolves
  failures/skips without exposing argv, stdout, stderr, or native documents.
- Marketplace refresh prerequisites do not set the daemon resource-change
  boolean, and the daemon never acknowledges partial operations.

## Ordering

Depends on the pure task graph. Completion unlocks isolated end-to-end
acceptance coverage.

## Implementation notes

Completed the daemon execution boundary without changing foreground lifecycle
behavior. Daemon native refresh and plugin update tasks now become one
validated `Plan`; exact marketplace prerequisites are attached with
`OperationDependency`, native and managed project routes share the existing
hybrid port, and one configuration lock/journal executes the batch. Current
cycle planning always creates fresh update operations instead of using prior
journal presence as a recovery no-op. Failed refreshes are journaled by the
core executor and dependent plugin operations are recorded as dependency skips,
while independent branches continue.

Native list observations now preserve one validated opaque `version` or
`revision` scalar. Bound daemon requests re-observe their precondition under
the lock and classify only equal present revisions as `NoChange`; changed or
absent revision evidence remains `Applied` after the existing postcondition.
Foreground constructors retain their prior behavior with no pre-revision
binding.

Daemon run state now stores validated typed operation references through a
private wire DTO. Status resolves each reference against the exact
resource/target `ApplyRecord`, rendering only phase, resource, target, result,
and bounded dependency IDs; missing journal evidence is reported as pending
rather than success. No native argv, process output, or payload is persisted.

The foreground lifecycle implementation was intentionally left behaviorally
unchanged rather than broad-refactored in this checkpoint; the daemon planner
reuses its existing capability selection, managed projection, lifecycle
operation constructors, execution port, lock, and journal boundaries.

## Verification

- `cargo fmt --check` — passed.
- `cargo test -p skilltap-core --lib` — 338 passed.
- `cargo test -p skilltap-harnesses --lib` — 27 passed.
- `cargo test -p skilltap --lib` — 69 passed.
- `cargo test -p skilltap-harnesses --test lifecycle_scope` — 2 passed.
- `cargo test -p skilltap --test native_postconditions` — 10 passed.
- `cargo test -p skilltap --test compiled_binary` — 50 passed.
- Strict workspace clippy remains blocked by two lint findings in the already-terminal task-graph file `crates/core/src/daemon.rs`; no checkpoint-2 file produced a clippy finding.
