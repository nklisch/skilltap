---
id: epic-rust-control-plane-runtime-maintainability-publication-tests
kind: story
stage: implementing
tags: [refactor, testing]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: [epic-rust-control-plane-runtime-maintainability-sidecar-tests]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Split Publication Recovery Scenarios

Replace the single six-scenario publication failure test with focused tests for
clean rollback, pre-publication temp residue, destination-only residue,
temporary-only residue, both residuals, and sync-only uncertainty. Preserve the
existing injected seam and assertions, add no abstraction without three uses,
and run the full locked verification ladder.
