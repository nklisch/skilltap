---
id: epic-cross-harness-materialization-publish-batch
kind: story
stage: implementing
tags: []
parent: epic-cross-harness-materialization-publish
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
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
