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

## Review findings (2026-07-12)

- **Blocker — plan drops typed detection next actions**: the reconciliation
  application copies observation resources and warnings but omits
  `observation.next_actions`. Plan output consequently lacks the exact
  category-specific recovery command required by this item's acceptance.
  Project the typed actions through reconciliation and cover plan in the
  isolated diagnostic matrix.
- **Blocker — post-mutation lifecycle drops typed next actions**: fresh
  lifecycle observation forwards warnings but not target-specific recovery
  actions, leaving only generic verification guidance.
- **Blocker — recovery command does not name the configured executable**:
  invalid/nonzero/bounded failures format `<harness> --version` from the
  harness ID rather than the absolute or custom configured binary that was
  actually invoked. Carry that safe executable identity through the mapper and
  cover custom configuration.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: plan and post-mutation lifecycle must preserve typed detection
next actions; recovery must name the configured executable
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the project-default `standard` weight.
The implemented lifecycle/status/adopt behavior and safety redaction are green;
the missing plan projection prevents approval and parent roll-up.

## Review repair (2026-07-12)

- Reconciliation now carries the native observation's typed next actions into
  plan and sync output.
- Fresh post-mutation observation carries the same typed actions instead of
  replacing them with generic verification guidance.
- Detection diagnostics receive the configured binary identity. Version
  recovery commands therefore inspect the exact configured executable, with
  safe shell display quoting when needed.
- The compiled failure matrix now verifies lifecycle and plan parity plus the
  exact custom executable command without exposing native output, argv, or
  environment content.

## Repair verification

- `cargo test -p skilltap application::tests::detection_diagnostics_are_typed_actionable_and_source_free --lib`
- `cargo test -p skilltap --test compiled_binary native_lifecycle_projects_each_detection_failure_without_sensitive_context -- --exact`
