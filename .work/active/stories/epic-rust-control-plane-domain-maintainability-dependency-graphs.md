---
id: epic-rust-control-plane-domain-maintainability-dependency-graphs
kind: story
stage: review
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

## Implementation notes

- Files changed: `crates/core/src/domain/dependency_graph.rs`, private module registration in
  `crates/core/src/domain/mod.rs`, graph adapters in `resource.rs` and `operation.rs`, and their
  existing test submodules.
- Tests added: resource/component/observed first-cycle selection across multiple disjoint cycles;
  operation all-cyclic-member selection across multiple disjoint cycles with a downstream node.
- Discrepancies from design: none. The shared support remains private; resource-family callers map
  the first exact cycle while operations map all actual cyclic members, preserving their distinct
  public semantics and validation order.
- Verification: locked workspace format, all-target check, Clippy with warnings denied, tests, and
  rustdoc with warnings denied all pass (58 core tests).
- Adjacent issues parked: none.
