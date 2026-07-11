---
id: epic-harness-observation-adoption-detection-probes
kind: story
stage: done
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

## Implementation

- Added strict bounded `probe_profile` execution and `narrow_profile` payload
  validation in `skilltap-harnesses`. Probe responses pass through the native
  process and JSON ports, scope is explicit, and profile narrowing delegates to
  the domain monotonicity contract.
- Unknown capabilities, widening support, invalid scopes/support values,
  malformed JSON, non-zero exits, and runtime failures remain typed probe
  errors; a verified profile id is preserved and unknown versions cannot gain
  authority.
- Added tests for project-scope narrowing, drift rejection, and real fixture
  probe execution.

## Verification

- Harness detection Clippy and all five detection tests pass in the locked
  offline workspace.

## Review

- Fast-lane review approved the green probe implementation. Probe authority is
  monotonic and remains separate from native mutation or persisted state.
