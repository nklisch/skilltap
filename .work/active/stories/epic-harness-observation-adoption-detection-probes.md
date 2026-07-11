---
id: epic-harness-observation-adoption-detection-probes
kind: story
stage: implementing
tags: [infra,correctness]
parent: epic-harness-observation-adoption-detection
depends_on: [epic-harness-observation-adoption-detection-profiles]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Narrow Detection With Read-Only Probes

Run optional adapter probes only for reachable installations through the
bounded process and strict JSON ports. Validate a probe response against the
compiled profile, narrow capabilities independently for global and project
scope, and reject widening, unknown capabilities, scope mismatches, duplicate
fields, malformed payloads, timeout/overflow, and executable replacement as
safe findings. Probes never grant unknown-version mutation authority and never
write native or skilltap state.
