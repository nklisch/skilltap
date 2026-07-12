---
id: feature-split-status-application
kind: feature
stage: drafting
tags: [refactor]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Split the StatusApplication god module

## Brief

Split the 6.8k-line `StatusApplication` implementation in
`crates/cli/src/application.rs` into private responsibility modules without
changing public signatures, output schemas, command behavior, or ownership
boundaries. The current module combines execution ports, daemon cycles,
marketplace/plugin lifecycle, skills, instructions, reconciliation,
adoption/status, and helper functions.

## Refactor constraints

- Pure behavior-preserving extraction only; no API or output changes.
- Keep composition and dependency wiring explicit at the CLI boundary.
- Preserve all existing tests and add no compatibility layer.
- Candidate boundaries: execution ports; lifecycle/skills; instructions and
  reconciliation; status/adoption.

## Acceptance

- Each extracted module has a coherent private responsibility.
- Existing workspace tests, formatting, and clippy remain green.
- Public command signatures and output remain byte/schema compatible.

