---
id: gate-tests-managed-terminal-journal-recovery
kind: story
stage: review
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
