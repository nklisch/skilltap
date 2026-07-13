---
id: epic-real-harness-recovery-runtime-boundary-diagnostics
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

# Project actionable detection diagnostics

## Scope

Carry closed detection failure categories through harness list, first-use and
configured status, planning, and lifecycle capability lookup, with actionable
safe output and isolated compiled-binary coverage.

## Acceptance

- Every public detection surface agrees on reachability and failure kind.
- Invalid version output is distinguishable from an absent binary.
- JSON/plain output contains no native stdout, argv, environment values, or
  secrets and gives the next command appropriate to the failure.
- Isolated roots remain unchanged during every read-only scenario.

## Implementation

- Added one closed diagnostic mapper for absent executables, invalid version
  responses, nonzero version commands, bounded failures, and other safe runtime
  failures.
- Harness list, first-use status, and configured status now preserve those
  categories without exposing stdout, argv, environment values, or runtime
  debug text.
- Each category includes a concrete safe next command: configure the harness
  binary when absent/unusable or inspect the exact harness version command.
- Native observation carries target-specific next actions alongside warnings;
  generic observation guidance remains as the final fallback.

## Verification

- Unit coverage asserts distinct stable warning/action codes and source-free
  projections.
- Compiled first-use status remains read-only under an isolated executable
  search path.
- `cargo test -p skilltap`
- `cargo clippy -p skilltap --all-targets --all-features -- -D warnings`

## Review findings (2026-07-12)

- **Blocker — lifecycle capability lookup still collapses typed detection failures**: `configured_native_profile` converts `detect_configured_installation` with `.ok()?`; its caller emits generic `native_profile_unavailable`. Lifecycle commands therefore do not expose the same absent/nonzero/invalid/bounded category required by harness list and status. Tracked by `epic-real-harness-recovery-runtime-boundary-diagnostics-completion`.
- **Blocker — compiled diagnostics regressions are stale**: the exact second-wave baseline fails `adopt_reports_partial_sibling_and_still_publishes_healthy_candidates` and `status_preserves_successful_sibling_observation_and_never_mutates_native_trees` because both still require the removed generic `native_detection_failed` code. The typed replacement behavior needs exact assertions on warning and next action.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: `epic-real-harness-recovery-runtime-boundary-diagnostics-completion`
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the project-default `standard` weight for the public diagnostic contract. Commit `ac89e49` correctly centralizes safe projections for harness list and observation/status paths, but does not carry the typed error through lifecycle capability lookup and leaves two compiled regressions red. Verification used a detached second-wave worktree to avoid concurrent third-wave edits.

## Bounce resolution (2026-07-12)

- `configured_native_profile` now preserves `DetectionError` through lifecycle
  capability lookup. Lifecycle output uses the same authoritative mapper as
  harness list and status while unsupported profiles/capabilities retain their
  distinct warnings.
- Partial adoption now projects the observation's typed next actions, matching
  status and plan behavior.
- The stale sibling status/adoption assertions now require the exact invalid
  version warning and `claude --version` next action.
- Compiled coverage exercises absent, invalid, nonzero, and bounded lifecycle
  detection without generic warnings or sensitive context.
- Integrated by `b0e1869`; the full workspace suite and all-feature Clippy are
  green, and the three diagnostics-focused compiled tests pass independently.
