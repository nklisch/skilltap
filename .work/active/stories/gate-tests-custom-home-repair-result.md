---
id: gate-tests-custom-home-repair-result
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

# Require completed output after successful custom-home repair

## Priority
High

## Spec reference
`epic-real-harness-recovery-filesystem-instructions-repair-completion`:
successful repair completes after exact postconditions hold.

## Required test
Require exit 0/completed and a repeated exit 0 no-op rather than permitting
attention-required.

## Implementation

- Tightened the isolated custom-`CODEX_HOME` regression to require exit 0 and
  `completed` after repair.
- Repeated the same repair and required exit 0, `completed`, and
  `summary.changed = false`.

## Verification

- `cargo test -p skilltap --test instruction_bridges custom_codex_home_uses_and_validates_the_effective_canonical_target -- --exact --nocapture`
