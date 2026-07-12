---
id: epic-cross-harness-materialization-publish-batch
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-publish
depends_on: []
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Plan Deterministic Publication Batches

Add the pure publication entry and batch contracts. Validate complete artifact
trees, duplicate resource/target pairs, and deterministic ordering before any
filesystem or harness action.

Acceptance criteria:

- Empty or malformed trees fail before publication.
- Entries sort deterministically by scope-bearing resource and target.
- A skill publication retains the complete directory including top-level
  `SKILL.md`.

## Implementation Notes

- Added `PublicationEntry`, `PublicationBatch`, and `plan_publication` in
  `crates/core/src/publication.rs`.
- Batches reject empty input, backup-role entries, and duplicate
  resource/target pairs before any side effect; entries are sorted by exact
  scope-bearing resource and target.
- Verification: targeted publication tests and core clippy passed.

## Review Record

- Inline review: **pass**. The contract is pure, deterministic, and reuses the
  existing complete-tree `ArtifactTree` boundary.
