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
