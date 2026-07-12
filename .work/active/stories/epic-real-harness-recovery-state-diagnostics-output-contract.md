---
id: epic-real-harness-recovery-state-diagnostics-output-contract
kind: story
stage: implementing
tags: [correctness, testing, documentation]
parent: epic-real-harness-recovery-state-diagnostics
depends_on:
  - epic-real-harness-recovery-state-diagnostics-dual-native-lifecycle
  - epic-real-harness-recovery-state-diagnostics-update-eligibility
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Align help and diagnostic aggregation

## Scope

Make exact plugin removal grammar discoverable, deduplicate exact next actions
at outcome merge boundaries, and carry repaired adapter diagnostics through
post-mutation output without replacing them with generic errors. Roll SPEC and
UX examples forward with the executable contract.

## Acceptance

- Plugin removal help and parsing require `PLUGIN@MARKETPLACE` consistently in
  root/group/leaf, plain/JSON, SPEC, and UX surfaces.
- Exact duplicate actions render once in first-seen order; materially distinct
  commands or explanations remain.
- Ordered normalization is idempotent and does not alter result or exit class.
- Native post-mutation failures retain the typed boundary reason and one
  actionable recovery command; healthy final observation completes normally.
- Multi-target/all-scope compiled-binary tests prove plain/JSON parity.
