---
id: gate-cruft-share-projection-helpers
kind: story
stage: done
tags: [cleanup]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: cruft
created: 2026-07-15
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

## Verification

Reused shared evidence, tree-limit, and optional-file helpers for Amp and Junie without changing byte limits or MCP error diagnostics.

- `cargo test -p skilltap-harnesses`: 163 passed.
- `cargo clippy -p skilltap-harnesses --all-targets -- -D warnings`: clean.
- Independent standard review: no material findings.
- `git diff --check`: clean.
