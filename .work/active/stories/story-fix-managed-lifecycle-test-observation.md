---
id: story-fix-managed-lifecycle-test-observation
kind: story
stage: done
tags: [bug, testing]
parent: null
depends_on: []
release_binding: 3.0.3
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Honor disabled native observation in managed lifecycle tests

## Symptom

Three managed-project application tests pass on developer machines with Codex
installed but fail on clean Linux and macOS CI. Their successful managed
operation is followed by an unrelated system observation that reports the
configured Codex executable missing.

## Root cause

`execute_native_lifecycle` invokes `NativeObservation::run` unconditionally
after execution, unlike status and reconciliation. It ignores the
`NativeObservationMode::Disabled` fixture boundary already selected by these
isolated tests.

## Fix approach

Use the application service's observation mode consistently: production
composition remains `System`, while isolated application tests can disable
ambient harness discovery and assert only the managed lifecycle under test.

## Regression test

Run the three existing managed lifecycle regressions with `PATH` set to an
empty isolated directory and require their success/recovery assertions to pass.

## Implementation notes

- Reused the existing `NativeObservationMode` boundary instead of adding a new
  fixture flag or fake harness adapter. Production composition remains
  `System`; only callers that explicitly select `Disabled` skip ambient native
  discovery.
- The three CI-failing managed-project regressions pass with a PATH containing
  only Git and no Codex executable.
- Focused application tests, formatting, and strict all-target/all-feature CLI
  Clippy pass.
- Effective review weight: standard, from the project default.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none
**Rejected**: none

**Notes**: Substrate review at explicit `standard` weight, escalated from the
fast story lane to a focused fresh-context deep pass because the change touches
post-mutation correctness. Production composition still constructs
`NativeObservationMode::System`; every `Disabled` construction is test-only.
The three affected managed-project regressions passed from the compiled test
binary with an isolated `PATH` containing Git and no Codex executable. All ten
compiled native-postcondition tests also passed, preserving required normal-CLI
post-mutation observation. `cargo fmt --all -- --check` and strict all-target,
all-feature CLI Clippy passed. Security, public-contract, foundation-doc,
naming, concurrency, persistence, and release lenses found no issue; the change
is a narrow reuse of the existing test observation boundary.
