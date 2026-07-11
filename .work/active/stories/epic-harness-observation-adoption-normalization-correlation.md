---
id: epic-harness-observation-adoption-normalization-correlation
kind: story
stage: implementing
tags: [correctness]
parent: epic-harness-observation-adoption-normalization
depends_on: [epic-harness-observation-adoption-normalization-graph]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Correlate Native Lineage Conservatively

Associate declared/effective instances only from a common declared source plus
compatible semantics or an explicit mapping. Preserve qualified identities and
layers; names, URLs, copied fingerprints, and cache coincidence must remain
non-equivalent without source evidence.
