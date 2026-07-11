---
id: epic-cross-harness-materialization-graph-contract
kind: story
stage: review
tags: []
parent: epic-cross-harness-materialization-graph
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define and Normalize Source Component Graphs

Implement `crates/core/src/plugin_graph.rs` with the typed declaration,
provenance, graph, reader port, and normalizer contracts described by the
parent feature.

Acceptance criteria:

- Duplicate, dangling, self-referential, and cyclic dependencies fail fast.
- Relative paths and declared names are validated at the boundary.
- Graph and provenance ordering is deterministic and source bytes are not
  retained.
- Unit tests cover all typed error branches and stable serialization-facing
  accessors.

## Implementation notes

- Files changed: `crates/core/src/plugin_graph.rs`, `crates/core/src/lib.rs`.
- Tests added: deterministic provenance ordering, dependency validation,
  duplicate IDs, and declared-name boundary tests in `plugin_graph.rs`.
- Discrepancies from design: `NativeId`'s existing opaque validation is used
  for declared names, so valid native names may contain spaces; surrounding
  whitespace and control characters still fail fast.
- Adjacent issues parked: none.
