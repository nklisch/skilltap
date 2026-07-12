---
id: epic-harness-observation-adoption-normalization-correlation
kind: story
stage: done
tags: [correctness]
parent: epic-harness-observation-adoption-normalization
depends_on: [epic-harness-observation-adoption-normalization-graph]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Correlate Native Lineage Conservatively

Associate declared/effective instances only from a common declared source plus
compatible semantics or an explicit mapping. Preserve qualified identities and
layers; names, URLs, copied fingerprints, and cache coincidence must remain
non-equivalent without source evidence.

## Implementation

- Added `conservatively_equivalent`, requiring a common declared source,
  matching resource kind, and matching component semantics. Missing sources,
  copied fingerprints, names, and URLs never correlate resources.

## Verification

- Harness Clippy and the locked normalization/runtime suites pass.

## Review

- Fast-lane review approved the source-and-semantics-only equivalence rule.
