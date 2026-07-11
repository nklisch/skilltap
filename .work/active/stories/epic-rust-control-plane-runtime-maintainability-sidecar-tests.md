---
id: epic-rust-control-plane-runtime-maintainability-sidecar-tests
kind: story
stage: implementing
tags: [refactor]
parent: epic-rust-control-plane-runtime-maintainability
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Move Runtime Tests to Sidecars

Mechanically move the complete `filesystem` and `scope` private test modules to
`runtime/filesystem/tests.rs` and `runtime/scope/tests.rs`. Preserve module
names, private access, all source text, all 22 fully qualified test identities,
and the 93-test inventory. Run the full locked verification ladder.
