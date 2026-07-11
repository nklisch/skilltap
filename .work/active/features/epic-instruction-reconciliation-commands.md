---
id: epic-instruction-reconciliation-commands
kind: feature
stage: implementing
tags: []
parent: epic-instruction-reconciliation
depends_on: [epic-instruction-reconciliation-repair]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Expose Instruction Commands

Expose deterministic setup/status/repair commands with project/global scope,
bridge mode, exact acknowledgment, and structured health output.

## Acceptance

Status is read-only; setup/repair fail fast on conflicts and emit actionable
next steps in plain and JSON modes.

## Design

- `instructions status` is read-only and scope-aware.
- Setup/repair must use canonical `~/AGENTS.md`, exact project paths, bridge
  mode, lock/revalidation, and recoverable backups before publication.
- Divergence requires an exact acknowledgment; generic confirmation must not
  bypass the repair plan.

## Implementation notes

`instructions status` now exposes a deterministic modeled-scope report and
explicitly reports that native bridge probing is pending. Setup currently emits
the same safe operation preview path as other mutators; filesystem publication
remains gated on the repair adapter.
