---
id: epic-harness-observation-adoption-normalization-integration
kind: story
stage: done
tags: [testing,infra]
parent: epic-harness-observation-adoption-normalization
depends_on: [epic-harness-observation-adoption-normalization-graph, epic-harness-observation-adoption-normalization-correlation, epic-harness-observation-adoption-normalization-findings]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Verify Native Normalization

Exercise repeated normalization, exact scope/layer preservation, conservative
cross-harness non-equivalence, partial sibling success, unresolved/malformed
findings, and safe deterministic output without writing state or native trees.

## Implementation

- Added normalization integration coverage for deterministic empty snapshots,
  exact target preservation through domain contracts, and health summaries that
  keep partial sibling failures visible.
- Conservative source/semantics correlation and strict child domain contracts
  are exercised alongside Codex/Claude adapter evidence suites.

## Verification

- Harness Clippy, normalization tests, and the locked workspace suite pass.

## Review

- Fast-lane review approved the deterministic normalization integration record.
