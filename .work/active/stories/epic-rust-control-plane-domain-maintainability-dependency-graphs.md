---
id: epic-rust-control-plane-domain-maintainability-dependency-graphs
kind: story
stage: implementing
tags: [refactor]
parent: epic-rust-control-plane-domain-maintainability
depends_on: [epic-rust-control-plane-domain-maintainability-validated-newtypes]
release_binding: null
gate_origin: refactor-design
created: 2026-07-11
updated: 2026-07-11
---

# Share Private Dependency Graph Traversal

Extract private known-reference, self-edge, and exact-cycle traversal primitives
used by resource/component/observation and operation graphs. Preserve every
public error variant, message, exact member-set semantic, serde rejection, and
test. Add equivalence coverage for downstream non-cycle nodes and multiple
cycles. Export no generic graph API; run the full locked workspace ladder.
