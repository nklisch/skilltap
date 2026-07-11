---
id: epic-rust-control-plane-domain-maintainability-operation-tests
kind: story
stage: implementing
tags: [refactor]
parent: epic-rust-control-plane-domain-maintainability
depends_on: []
release_binding: null
gate_origin: refactor-design
created: 2026-07-11
updated: 2026-07-11
---

# Externalize Operation Domain Tests

Mechanically move the inline `operation.rs` test module to
`operation/tests.rs`. Preserve every test, fixture, assertion, name, and private
item access. Production code, public API, serde forms, and behavior must not
change. Verify the same 56 core tests plus locked workspace checks.
