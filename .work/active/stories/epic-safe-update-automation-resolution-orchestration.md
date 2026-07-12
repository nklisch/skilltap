---
id: epic-safe-update-automation-resolution-orchestration
kind: story
stage: done
tags: []
parent: epic-safe-update-automation-resolution
depends_on: [epic-safe-update-automation-resolution-adapters]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Check and Cache Available Revisions

Wire the pure resolution contracts into the application status/check path and
add atomic available-revision caching in `state.json` without mutating desired
inventory, managed artifacts, or native harness files.

Acceptance criteria:

- Repeating an unchanged check is a no-op and reports no update.
- Changed Git SHAs and native revisions are visible before any update action.
- Failures leave state, inventory, and native configuration unchanged.
- Successful cache writes preserve existing operation journals and emit the
  documented human/JSON next actions.

## Implementation notes

- Files changed: `crates/core/src/updates.rs`,
  `crates/core/src/storage/state.rs`, `crates/core/src/storage/tests.rs`, and
  `crates/cli/src/application.rs`.
- Tests added: atomic available-revision cache preservation and existing core,
  CLI, and compiled-binary status coverage remained green.
- Discrepancies from design: status performs read-only candidate projection and
  intentionally does not publish `state.json`; the new state method is the
  pure atomic cache primitive for the foreground/daemon writer. The current
  daemon command remains unavailable until its service feature consumes this
  primitive.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core storage::tests::available_revision_cache_preserves_apply_history_and_siblings --offline`
  — passed.
- `cargo test -p skilltap --offline` — passed.
- `cargo clippy -p skilltap --all-targets --offline -- -D warnings` — passed.

## Review

Verdict: Approve — story verified by implement; fast-lane advance.
