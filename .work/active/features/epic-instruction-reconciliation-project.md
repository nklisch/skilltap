---
id: epic-instruction-reconciliation-project
kind: feature
stage: done
tags: []
parent: epic-instruction-reconciliation
depends_on: [epic-instruction-reconciliation-model]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Manage Project Instruction Bridges

Manage project-root and explicitly adopted nested instructions, preserving the
single project bridge rule and distinguishing root versus `.claude/CLAUDE.md`.

## Acceptance

Both-entry conflicts warn and block ordinary repair; approved consolidation
backs up divergent content and leaves one managed root bridge.

## Implementation notes

Project path derivation now shares the canonical instruction model and keeps
root `AGENTS.md`/`CLAUDE.md` locations scope-exact. Duplicate-entry handling is
left visible for the repair planner rather than silently selecting one file.

## Review

### Verdict

Approve with comments.

### Findings

- Nested discovery and duplicate root versus `.claude/CLAUDE.md` observation
  still belong to the repair adapter.

### Verification

Project path and instruction model tests pass under strict clippy.
