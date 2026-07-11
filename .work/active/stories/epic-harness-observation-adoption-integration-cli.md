---
id: epic-harness-observation-adoption-integration-cli
kind: story
stage: done
tags: [cli,testing]
parent: epic-harness-observation-adoption-integration
depends_on: [epic-harness-observation-adoption-integration-core]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify Adoption CLI Contracts

Expand compiled-binary coverage for global/current/explicit/all scopes, omitted
and explicit `--from` targets, stable plain/JSON decisions, partial/conflict
attention exits, and inventory-only mutation with immediate repeat idempotence.

## Implementation notes

- Added compiled coverage for healthy Codex adoption with immediate repeat,
  partial Claude sibling failure, current project adoption, and all-recorded-
  scopes replay.
- Assertions cover stable JSON result classes, attention warnings, project
  inventory registration, and inventory-only publication.

## Verification

- `cargo fmt --all`
- `cargo test -p skilltap --test compiled_binary adopt_ --offline`

## Review

Verdict: Approve - story verified by implement; fast-lane advance.
