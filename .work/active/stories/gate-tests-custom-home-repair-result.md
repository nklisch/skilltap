---
id: gate-tests-custom-home-repair-result
kind: story
stage: done
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

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none
**Rejected**: none

**Notes**: Substrate review at effective weight `standard` (caller), Standard lane because the assertions cover a custom-home path contract. The tightened test now rejects the former ambiguous exit, proves the successful repair reports `completed`, and proves the immediate repeat is an exit-0 no-op. The exact isolated test passes in a detached clean worktree; no production or foundation-doc change was introduced by this item.
