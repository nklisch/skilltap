---
id: epic-instruction-reconciliation-commands
kind: feature
stage: drafting
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
