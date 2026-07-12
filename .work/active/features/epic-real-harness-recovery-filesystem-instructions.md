---
id: epic-real-harness-recovery-filesystem-instructions
kind: feature
stage: drafting
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-and-adapter-expansion
depends_on: [epic-real-harness-recovery-runtime-boundary]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Preserve skill executability and correct instruction bridges

## Brief

Preserve safe executable semantics for complete skill directories and compute
instruction bridge targets relative to the actual canonical file for arbitrary
supported `HOME`/`CODEX_HOME` layouts. Health checks must resolve and compare
the effective link target, and an acknowledged repair that leaves no blocker
must complete successfully after creating its recoverable backup.

This feature owns blocker inventory entries 13-15. It does not change native
plugin lifecycle or aggregate update/status projection.

## Epic context

- Parent epic: `epic-real-harness-recovery-and-adapter-expansion`
- Position in epic: consumes the shared runtime/root model; state/diagnostics
  consumes its final result semantics.

## Foundation references

- `docs/SPEC.md` — standalone skill model, instruction lifecycle, and mutation
  safety.
- `docs/ARCH.md` — standalone skills and instruction management.
- `docs/HARNESS-CONTRACTS.md` — canonical global instruction bridges.
