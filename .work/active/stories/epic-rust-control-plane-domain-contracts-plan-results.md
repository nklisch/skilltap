---
id: epic-rust-control-plane-domain-contracts-plan-results
kind: story
stage: implementing
tags: []
parent: epic-rust-control-plane-domain-contracts
depends_on: [epic-rust-control-plane-domain-contracts-resource-graph, epic-rust-control-plane-domain-contracts-capability-compatibility]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Plans, Operation Dependencies, and Results

## Scope

Implement Unit 4 from the parent feature: serializable operation graphs,
classifications, reversibility, exact piecewise acknowledgment requirements,
attention reasons, and validated per-operation and final apply outcomes.

## Acceptance criteria

- [ ] Plan construction rejects duplicate ids, unknown dependencies, and cycles.
- [ ] Partial operations enumerate exact selectors and non-empty consequences;
  no generic confirmation value erases compatibility evidence.
- [ ] Apply results cannot claim success when an operation failed or remains
  blocked, and dependent skips remain distinguishable from failures.
- [ ] Representative plans and partial results serialize deterministically and
  round-trip through JSON.
- [ ] No planner, executor, adapter, persistence, or CLI rendering behavior is
  introduced.
- [ ] Locked format, clippy, and workspace tests pass.
