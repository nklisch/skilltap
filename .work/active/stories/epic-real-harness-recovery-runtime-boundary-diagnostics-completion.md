---
id: epic-real-harness-recovery-runtime-boundary-diagnostics-completion
kind: story
stage: review
tags: [correctness, testing]
parent: epic-real-harness-recovery-runtime-boundary
depends_on:
  - epic-real-harness-recovery-runtime-boundary-process-context
  - epic-real-harness-recovery-runtime-boundary-version-decoding
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Complete typed diagnostics across lifecycle surfaces

## Finding

Harness list and status use the closed detection diagnostic mapper, but native
lifecycle capability lookup discards `DetectionError` and emits a generic
profile warning. Two compiled tests also retain assertions for the removed
generic warning code, leaving the exact implementation baseline red.

## Required fix

- Return the typed detection failure from lifecycle capability lookup and
  project it through the same authoritative warning/next-action mapper used by
  harness list and status.
- Keep an unsupported exact profile or unverified capability distinct from a
  failed version detection.
- Update the two stale sibling-observation/adoption compiled tests to assert
  the exact typed category and safe next action.
- Add lifecycle compiled coverage for absent executable, invalid version,
  nonzero version command, and bounded failure without raw stdout, argv,
  environment, or secret leakage.
- Run the full all-target workspace suite, formatting, and all-feature clippy.

## Acceptance

- Harness list, status/adopt/plan, and lifecycle commands agree on the same
  detection category and next action.
- Unsupported profile/capability remains a separate post-detection outcome.
- The full workspace suite is green.

## Implementation notes

- Execution capability: focused inline implementation coordinated through the
  target-evidence integration owner because the state-schema migration shared
  `application.rs`, lifecycle handling, status projection, and compiled tests.
- Review weight: standard (project default).
- Integration commit: `b0e1869`.
- Files changed: `crates/cli/src/application.rs`,
  `crates/cli/src/application/lifecycle.rs`,
  `crates/cli/src/application/status.rs`, and
  `crates/cli/tests/compiled_binary.rs` within the atomic integration commit.
- Tests added: one compiled lifecycle matrix for missing executable, malformed
  version, nonzero version command, and bounded output; exact status and adopt
  assertions for typed warning codes and safe next actions.
- Discrepancies from design: partial adoption also needed to project the
  observation's typed next actions; status already did so. This was fixed in
  the same authoritative projection path rather than weakening the stale test.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap --test compiled_binary native_lifecycle_projects_each_detection_failure_without_sensitive_context -- --exact`
- `cargo test -p skilltap --test compiled_binary status_preserves_successful_sibling_observation_and_never_mutates_native_trees -- --exact`
- `cargo test -p skilltap --test compiled_binary adopt_reports_partial_sibling_and_still_publishes_healthy_candidates -- --exact`
- Integration commit `b0e1869`: `cargo test --workspace --all-targets` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
