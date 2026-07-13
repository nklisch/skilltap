---
id: epic-real-harness-recovery-filesystem-instructions-repair-outcome
kind: story
stage: review
tags: [correctness, testing]
parent: epic-real-harness-recovery-filesystem-instructions
depends_on:
  - epic-real-harness-recovery-filesystem-instructions-relative-bridges
release_binding: null
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
