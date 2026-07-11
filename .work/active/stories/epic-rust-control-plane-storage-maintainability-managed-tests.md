---
id: epic-rust-control-plane-storage-maintainability-managed-tests
kind: story
stage: implementing
tags: [refactor, testing]
parent: epic-rust-control-plane-storage-maintainability
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Split Managed Artifact Tests by Contract

Mechanically split the managed-artifact sidecar into tree-contract,
real-filesystem lifecycle/security, and failure-mapping modules with minimal
shared fixtures. Preserve test identities, assertions, and explicit fault
doubles. Record and compare the test list, then run the full locked ladder.
