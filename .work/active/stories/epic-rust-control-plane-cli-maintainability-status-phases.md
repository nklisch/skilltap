---
id: epic-rust-control-plane-cli-maintainability-status-phases
kind: story
stage: done
tags: [refactor]
parent: epic-rust-control-plane-cli-maintainability
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Decompose Foundation Status Phases

Extract private typed document-load, scope, target, and outcome-projection
phases from `StatusApplication::execute`. Keep its crate-visible signature and
all output bytes, error/resource ordering, early returns, and filesystem effects
unchanged. Add focused phase tests only where they improve contract clarity and
run the locked/binary ladders.

## Implementation notes

- Files changed: `crates/cli/src/application.rs`.
- Extracted private typed `DocumentLoadPhase`, `StatusDocuments`, `StatusScope`,
  `StatusTargets`, and `StatusProjection` boundaries while preserving the
  crate-visible `StatusApplication::execute` signature.
- Kept owned-document resource classification and errors in
  config/inventory/state order; scope still resolves before targets; target
  resources and summaries retain their original order; every original early
  return remains at the same phase boundary.
- Tests added: none. The existing application and compiled-binary contracts
  already exercise each phase and its representative failures.
- Behavior baseline: clean pre-refactor and refactored binaries produced
  identical stdout, stderr, and exit codes for global plain, global JSON,
  explicit-project JSON with a single target, and all-scopes plain status.
- Verification: locked format, workspace check, Clippy with warnings denied,
  192 workspace tests, rustdoc, optimized build, binary smoke verification, and
  the six-test compiled-binary suite all pass.
- Dispatch rationale: direct-read only; the implementation surface was one
  module with existing focused and binary-level coverage.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review

Approved. Typed private phases preserve document/resource/error ordering,
scope-before-target sequencing, early returns, output bytes, exits, and effects
while leaving the crate-visible application boundary unchanged.
