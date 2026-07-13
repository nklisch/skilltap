---
id: gate-tests-managed-terminal-journal-recovery
kind: story
stage: implementing
tags: [testing]
parent: null
depends_on: []
release_binding: 3.0.2
gate_origin: tests
created: 2026-07-12
updated: 2026-07-12
---

# Exercise terminal managed journal failure through lifecycle retry

## Priority
Critical

## Spec reference
`epic-real-harness-recovery-native-lifecycle-managed-project-journal-recovery`:
first install and update terminal journal failures retry as verified no-change
without duplicate projection publication.

## Required test
Use the real lifecycle with fail-on-terminal-write state storage and recording
publication, then assert exact Pending promotion and immediate repeat no-op.

## Implementation

- Added a Git-backed marketplace fixture and exercised terminal state journal
  failure through the public managed project lifecycle for both first install
  and a later commit update.
- Asserted the pending attempt retains the exact desired Git revision and
  sorted projection manifest while the previously confirmed state remains
  unchanged.
- Fixed retry planning to use a matching pending attempt's projection manifest
  as its observation baseline and normalize desired projection ordering before
  recovery validation.
- Verified retry promotes the pending revision and manifest without another
  tree publication, then an immediate repeat remains a no-op.

## Verification

- `cargo test -p skilltap --lib managed_terminal_journal_failure_recovers_without_duplicate_projection_publication`
- `cargo test -p skilltap --lib`

## Review findings

- **Blocker — exact projection-manifest promotion is claimed but not asserted.**
  The install and update checks at
  `crates/cli/src/application/tests.rs:648-655` and `729-736` require only a
  non-empty pending manifest; the recovered bindings at `677-680` and
  `762-765` likewise require only non-empty projections. This does not prove
  that both the skill and MCP projections are present, sorted/deduplicated, or
  promoted unchanged from the pending attempt. Assert the exact expected
  manifest (including order) for install and update and equality with the
  recovered binding, while retaining the publication-count checks.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: exact pending projection-manifest promotion is not asserted
**Important**: none
**Nits**: the recorded focused command uses `--exact` without the module-qualified test name and selects zero tests; use `cargo test -p skilltap --lib application::tests::managed_terminal_journal_failure_recovers_without_duplicate_projection_publication -- --exact`
**Rejected**: none

**Notes**: Substrate review at effective `standard` weight (caller-selected),
escalated to the Deep lane for terminal persistence and retry safety.
Same-harness fresh-context review inspected commit `e7dfc6a`, pending-attempt
construction, operation-journal ordering, ownership/recovery validation, Git
revision handling, and the lifecycle-level test. The test uses the actual
application lifecycle and repositories, fails the terminal state write after
real projection publication, and proves install/update retries do not publish
the tree again. Matching recovery requires Pending journal evidence plus exact
fingerprint, revision, operation id, and projection equality; the sibling
publication-failure story also demonstrates a failed apply is retried rather
than blessed. A clean-target `cargo test -p skilltap --lib` runs 70 tests and
passes. No public CLI, schema, security boundary, or foundation-doc assertion
changed in this story.
