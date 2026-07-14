---
id: epic-expanded-harness-support-candidate-admission-gate
kind: story
stage: implementing
tags: [testing]
parent: epic-expanded-harness-support-candidate-admission
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Define Candidate Admission Authority and Gate

## Checkpoint

Represent a known, version-pinned but deliberately read-only harness profile
without granting mutation authority, and define one dependency-neutral candidate
matrix used by Cursor, Zoo Code, and ZCode.

Add `CapabilityProfileSelection::VerifiedObserveOnly { id, capabilities }` in
`crates/core/src/domain/installation.rs`. It reports
`ProfileAuthority::ObserveOnly`, preserves a profile id and observation
capabilities, returns no mutation capabilities, and remains observe-only after
runtime narrowing. Existing verified-compiled and unknown-version behavior and
wire forms remain unchanged.

Add `crates/test-support/src/candidate_admission.rs` with the checks,
`CandidateDisposition::{Admitted, ObserveOnly, Blocked}`, evidence, report, and
`candidate_admission_gate` specified in the parent feature. The production-aware
candidate runner must perform concrete source/path/version/reload/ownership
assertions before returning a check; evidence labels alone do not pass the gate.

Disposition rules:

- `Admitted` requires every check.
- `ObserveOnly` requires exact deterministic installation identity and safe,
  documented read-only observation but is missing at least one mutation check.
- `Blocked` covers missing deterministic identity or safe observation.

The matrix is test support only. Production authority remains the ordinary
profile and optional adapter ports; no candidate executor or runtime disposition
switch is introduced.

## Acceptance evidence

- [ ] Verified observe-only profiles round-trip, expose their id and scoped
      observation capabilities, and never expose mutation capabilities.
- [ ] Narrowing cannot promote verified observe-only or unknown profiles.
- [ ] Every exhaustive profile renderer handles the new variant accurately.
- [ ] The candidate matrix rejects a false `Admitted` result with one missing
      check and a false `ObserveOnly` result without safe observation.
- [ ] Existing profile tests remain green without assertion weakening.
- [ ] No production executor imports `CandidateDisposition` or the test-support
      admission matrix.

## Ordering

Foundation checkpoint for all three boundary stories. It creates no candidate
adapter and grants no target authority by itself.
