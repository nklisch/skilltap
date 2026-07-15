---
id: epic-expanded-harness-support-candidate-admission-gate
kind: story
stage: done
tags: [testing]
parent: epic-expanded-harness-support-candidate-admission
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Define Candidate Admission Authority and Gate

## Checkpoint

Represent deliberately read-only candidate profiles without granting mutation
authority, and define one dependency-neutral candidate matrix used by Cursor,
Zoo Code, and ZCode. The 2026-07-15 relaxed amendment separates registry
observation from mutation admission.

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
- `ObserveOnly` requires reliable target identity plus at least one safe,
  source-documented read surface, and is missing exact mutation/effective checks.
  It may register a read-only adapter or typed file-only contract.
- `Blocked` covers missing reliable identity or safe documented observation.
- Exact compiled profile identity remains mandatory for every mutation channel;
  `--yes` and `VerifiedObserveOnly` never change that ceiling.

The matrix is test support only. Production authority remains the ordinary
profile and optional adapter ports; no candidate executor or runtime disposition
switch is introduced.

## Acceptance evidence

- [x] Verified observe-only profiles round-trip, expose their id and scoped
      observation capabilities, and never expose mutation capabilities.
- [x] Narrowing cannot promote verified observe-only or unknown profiles.
- [x] Every exhaustive profile renderer handles the new variant accurately.
- [x] The candidate matrix rejects a false `Admitted` result with one missing
      check and a false `ObserveOnly` result without safe observation.
- [x] Existing profile tests remain green without assertion weakening.
- [x] No production executor imports `CandidateDisposition` or the test-support
      admission matrix.

## Implementation Notes

- Added `VerifiedObserveOnly` and `verified_observe_only` in the core profile
  contract. It retains exact profile identity and scoped observation evidence,
  maps to `ProfileAuthority::ObserveOnly`, exposes no mutation capability set,
  and preserves its authority and identity through narrowing.
- Added the dependency-neutral test-support candidate matrix with sixteen
  checks, including `ReliableTargetIdentity` and
  `SafeDocumentedReadSurface`. The relaxed read-only set is intentionally only
  those two checks; exact installation identity, precedence/reload,
  preservation, ownership, and repeat checks remain mutation evidence.
- The gate remains test-support-only. Candidate adapters now consume a typed
  `ReadOnlyTargetPort` only for file-only/editor identity boundaries; no gate
  disposition is consulted by the executor. Existing renderers still render
  `ProfileAuthority::ObserveOnly` accurately.

## Verification

- Focused tests passed: `cargo test -p skilltap-core domain::installation`,
  `cargo test -p skilltap-test-support candidate_admission`,
  `cargo test -p skilltap entrypoint::tests`,
  `cargo test -p skilltap output::tests`, and
  `cargo test -p skilltap application::tests`.
- The original gate verification passed before the amendment. The relaxed
  implementation re-runs the workspace ladder after candidate registration;
  the final counts and commands are recorded in the parent feature and
  acceptance story. `git diff --check` and strict Clippy remain required before
  parent review.

## Ordering

Foundation checkpoint for all three boundary stories. It creates no candidate
adapter and grants no target authority by itself.
