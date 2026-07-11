---
id: epic-instruction-reconciliation-model
kind: feature
stage: done
tags: []
parent: epic-instruction-reconciliation
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Model Instruction Locations and Bridges

Define scope-bearing canonical instruction locations, fingerprints, ownership,
bridge mode, and health findings for global and project resources.

## Acceptance

The model distinguishes missing, managed, divergent, broken, duplicate, and
unmanaged instructions without reading raw authored content into findings.

## Implementation notes

Added `skilltap_core::instructions` with scope-neutral bridge mode, fingerprint,
and explicit health classification for missing, managed, divergent, and
unmanaged states.

## Review

### Verdict

Approve with comments.

### Findings

- Native path probing and duplicate-entry detection remain in the global and
  project adapter features.

### Verification

Focused instruction model tests and strict core clippy pass.
