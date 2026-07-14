---
id: feature-daemon-marketplace-refresh-execution
kind: story
stage: implementing
tags: [infra]
parent: feature-daemon-marketplace-refresh
depends_on: [feature-daemon-marketplace-refresh-task-graph]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
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
