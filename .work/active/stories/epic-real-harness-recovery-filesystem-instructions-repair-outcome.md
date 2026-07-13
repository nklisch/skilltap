---
id: epic-real-harness-recovery-filesystem-instructions-repair-outcome
kind: story
stage: done
tags: [correctness, testing]
parent: epic-real-harness-recovery-filesystem-instructions
depends_on:
  - epic-real-harness-recovery-filesystem-instructions-relative-bridges
  - epic-real-harness-recovery-filesystem-instructions-repair-completion
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Complete successful acknowledged instruction repairs

## Scope

Separate unresolved attention from repair disclosure and derive the final
instruction outcome from execution plus post-apply bridge and backup health.

## Acceptance

- Acknowledged divergent-file repair and target-scoped sync return exit 0 with
  `completed` after preserving the original bytes in a recoverable backup and
  producing a healthy bridge.
- Output discloses the backup without claiming a resolved decision still needs
  attention.
- Unacknowledged divergence, backup/apply failure, mixed-scope blockers, and
  failed post-observation remain attention-required and actionable.
- Repeating a successful repair creates no backup, reports no change, and exits
  successfully in plain and JSON output.

## Implementation

- Successful repair/consolidation warnings are treated as disclosures, while
  every other warning remains unresolved attention.
- Final result now requires successful operation outcomes plus fresh checks of
  the canonical file, exact symlink/import representation, removed duplicates,
  and any promised regular-file backup.
- Recoverable backups are projected as explicit `preserved` resources with
  their managed path, and regression coverage verifies the original bytes.
- Repeating the repaired command is a completed zero-change operation with no
  additional backup.

## Verification

- Focused compiled-binary global repair and repeat coverage.
- Existing project duplicate, mixed blocker, and broken-entry regressions.
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## Review findings (2026-07-12)

- **Blocker — import postcondition follows symlinks**:
  `instruction_entry_postcondition` validates an import with
  `FileSystem::read`, so a post-apply swap to a symlink whose destination has
  the expected bytes is accepted as healthy. This contradicts the exact
  regular-file import representation used by the authoritative bridge
  classifier and can incorrectly return `completed`. Validate with
  `read_regular_no_follow` and add a post-apply swap/fake-filesystem regression.
- **Blocker — acknowledged target-scoped sync remains attention-required**:
  direct `instructions repair` now completes, but reconciliation merges the
  repair disclosure warning and only normalizes when the aggregate warning
  list is empty. Existing global and project regressions still require exit 2
  after a successful `sync --yes`, contrary to this story's explicit exit-0
  acceptance. Reconciliation must distinguish resolved disclosures from
  unresolved warnings and assert completed global/project sync results.

Tracked by `epic-real-harness-recovery-filesystem-instructions-repair-completion`.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: `epic-real-harness-recovery-filesystem-instructions-repair-completion`
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the project-default `standard` weight,
escalated from Fast because this is a correctness/security-sensitive
filesystem postcondition. Focused repair, repeat, duplicate, and blocker tests
plus full workspace tests and all-feature Clippy are green, but the no-follow
representation gap prevents approval.

## Bounce resolution (2026-07-12)

- Import postconditions now use descriptor-bound `read_regular_no_follow` and
  reject symlinks even when their destination has the expected bytes.
- Reconciliation treats only successful instruction repair/consolidation
  disclosures as resolved; other warnings, observation failures, and errors
  remain attention-required.
- Global and project `sync --yes` now require completed exit-0 results after
  exact harness-specific observation and healthy repaired bridges.
- Repeat direct repair is covered in JSON and plain output and preserves the
  original single backup count.
- Implemented by
  `epic-real-harness-recovery-filesystem-instructions-repair-completion`.

## Review (2026-07-12, repair pass)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the project-default `standard` weight.
The corrective child closes both prior blockers. Focused isolated tests prove
acknowledged global/project repairs complete, preserve exactly one recoverable
backup, repeat as plain/JSON no-ops, and reject a matching-byte symlink.
