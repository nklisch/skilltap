---
id: epic-cross-harness-materialization-hooks-integration
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-hooks
depends_on: [epic-cross-harness-materialization-hooks-equivalence]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Hand Hook Analysis Into Reconciliation

Expose pure hook compatibility through reconciliation/materialization planning
without native registration or managed filesystem writes.

Acceptance criteria:

- Required hook mismatch blocks before publication.
- Optional hook loss is represented by exact partial selectors.
- Scope-bearing resource/component identity is preserved end to end.

## Implementation Notes

- Added `hook_compatibility_for_target` at the reconciliation boundary. The
  function delegates to the pure normalized hook analyzer and performs no
  native calls or filesystem publication.
- Added a reconciliation test proving a faithful scope-bearing handoff.
- Verification: `cargo test -p skilltap-core --offline` and
  `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings`.

## Review Record

- Inline fresh-context review: **pass**. The handoff is adapter-neutral,
  target-bound, and preserves the existing compatibility result contract.
- No follow-up item filed.
