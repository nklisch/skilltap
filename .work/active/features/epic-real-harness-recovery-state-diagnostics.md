---
id: epic-real-harness-recovery-state-diagnostics
kind: feature
stage: drafting
tags: [correctness, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on:
  - epic-real-harness-recovery-native-lifecycle
  - epic-real-harness-recovery-filesystem-instructions
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Make lifecycle state and diagnostics target-exact

## Brief

Correct update summaries, help contracts, next-action aggregation, sequential
target widening, and dual-native state so agents receive one precise and
actionable account of every target. A logical plugin published natively to both
harnesses must keep separate target bindings and lifecycle evidence, never a
managed copy, and narrowed operations must preserve the sibling target.

This feature owns blocker inventory entries 12 and 16-20 plus the generic
post-mutation diagnostic friction that remains after native adapter repair. It
consumes the final lifecycle and instruction result contracts rather than
papering over their failures in rendering.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: final integration and agent-facing correctness feature.

## Foundation references

- `docs/SPEC.md` — state, plugin lifecycle, output, and exit codes.
- `docs/ARCH.md` — domain model, planning, updates, and error model.
- `docs/VISION.md` — native-first ownership and agent-readable operation.

