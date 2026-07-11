---
id: epic-harness-observation-adoption-runtime-strict-json
kind: story
stage: implementing
tags: [infra,correctness]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Decode Strict Bounded Native JSON

Implement a byte-capped UTF-8 boundary that accepts exactly one JSON document
with trailing whitespace only. Use a recursive seed/visitor to reject duplicate
object keys at every depth and enforce an explicit nesting limit before typed
decode; do not rely on `serde_json::Value` last-key-wins behavior. Reject
trailing documents/garbage and invalid UTF-8 with fixed safe errors that never
echo native bytes or parser excerpts. Honor the contract's hard stack-safe
depth ceiling and test zero plus every hard limit at minus/at/plus one.
