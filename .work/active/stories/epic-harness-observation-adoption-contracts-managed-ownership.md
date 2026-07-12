---
id: epic-harness-observation-adoption-contracts-managed-ownership
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-key]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Migrate Managed Artifact Ownership

Use exact `ResourceKey` owners throughout managed records, repository ports,
handles, errors, residuals, serde, and canonical artifact/backup path hashing.
Prove same logical ID in different scopes never aliases and owner mismatch fails
before filesystem I/O.

## Implementation notes

- Files changed: managed artifact records, repository ports and file adapter,
  managed error/residual translation, strict managed schema errors, the minimal
  `ResourceState` key boundary, and focused managed artifact tests.
- Tests added: equal logical IDs across global and two project scopes derive
  distinct canonical artifact and backup paths; wrong-scope owners fail load
  and remove before any filesystem port method is called.
- Discrepancies from design: the `ResourceState` owner validation boundary had
  to migrate in this stride because it directly consumes managed records. The
  `InventoryDocument` in-memory index and undeclared-project diagnostic also
  had to accept the graph's exact keys to close the compile-time dependency
  cycle. The broader strict document wire, fixture, and golden reset remains in
  the dependent storage-wires story.
- Adjacent issues parked: none.

## Review

- Approved in the coordinated exact-key migration review.
- Verified distinct artifact and backup paths for equal logical IDs across
  scopes and exact owner rejection before filesystem I/O.
- Locked workspace format, all-target check, warnings-denied Clippy, and all
  209 tests pass with the storage bridge applied.
