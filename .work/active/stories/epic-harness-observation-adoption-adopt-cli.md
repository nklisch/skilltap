---
id: epic-harness-observation-adoption-adopt-cli
kind: story
stage: done
tags: [cli]
parent: epic-harness-observation-adoption-adopt
depends_on: [epic-harness-observation-adoption-adopt-persistence]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Expose Adoption CLI

Route `adopt` through exact scope/target selection and the locked application
service. Render typed adopted/coalesced/already-managed/conflict/unadoptable
decisions in stable plain/JSON output; partial/conflict results require the
documented acknowledgment and no generic bypass is introduced.

## Implementation notes

- Routed `adopt` through the existing deterministic config/inventory/scope and
  harness observation pipeline for global, project, all-scope, and explicit
  target selection.
- Added stable plain/JSON decision projection for adopted, coalesced,
  already-managed, conflict, and unadoptable outcomes, with attention on
  partial native observation and stale/locked publication errors.
- The command has no generic acknowledgment flag and does not mutate native
  harness configuration, state.json, or managed artifacts.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap --all-targets --offline`
- `cargo clippy --workspace --all-targets --offline -- -D warnings`

## Review

Verdict: Approve - story verified by implement; fast-lane advance.
