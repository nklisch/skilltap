---
id: gate-cruft-share-projection-helpers
kind: story
stage: implementing
tags: [cleanup]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: cruft
created: 2026-04-02
updated: 2026-07-15
---

# Share trust-interactive projection helpers

## Confidence
Medium

## Category
duplicated helper

## Location
`crates/harnesses/src/adapters/trust_interactive/amp_projection.rs:710`, `junie_projection.rs:347`, and `configuration_constrained/common.rs:222`

## Evidence
Amp and Junie projections duplicate `evidence`, `tree_limits`, and `read_optional_file` helpers already available beside the shared `plan_skills` they import.

## Removal
Reuse the common helpers, adapting the `JsonLimits` parameter without weakening limits or diagnostics, and delete local copies. Preserve projection output and observation behavior.
