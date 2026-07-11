---
id: epic-harness-observation-adoption-runtime-contracts-limits
kind: story
stage: implementing
tags: [infra]
parent: epic-harness-observation-adoption-runtime
depends_on: []
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Observation Runtime Limits and Ports

Add harness-neutral non-zero bounded request/limit/status contracts with hard
compile-time ceilings and checked cross-field relationships, behavior
ports for executable resolution, process execution, JSON decoding, and external
tree observation, plus a closed safe error taxonomy. Custom Debug/Display and
serde forms must never expose argv, environment values, native output, parser
excerpts, file bytes, or caller-provided raw paths. Keep concrete I/O out of the
contract module and avoid redefining installation or snapshot domain types.
Reject zero, hard-maximum overflow, allocation/counter/duration overflow, a
combined process cap below either stream cap, and a total tree cap below its
per-file cap. Cap JSON nesting at a documented stack-safe maximum and test
every limit at zero and maximum minus/at/plus one.
