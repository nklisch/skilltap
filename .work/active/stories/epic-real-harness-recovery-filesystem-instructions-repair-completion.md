---
id: epic-real-harness-recovery-filesystem-instructions-repair-completion
kind: story
stage: done
tags: [correctness, security, testing]
parent: epic-real-harness-recovery-filesystem-instructions
depends_on:
  - epic-real-harness-recovery-filesystem-instructions-relative-bridges
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Complete instruction repair postconditions and sync results

## Finding

Direct acknowledged repair now preserves backups and completes, but the import
postcondition follows symlinks and successful target-scoped reconciliation
still treats repair disclosures as unresolved attention.

## Required fix

- Validate import postconditions with `read_regular_no_follow` so only the
  exact regular-file representation can complete.
- Add a post-apply swap or fake-filesystem regression proving a symlink with
  matching target bytes remains attention-required.
- Teach reconciliation aggregation to distinguish resolved instruction repair
  disclosures from unresolved warnings without suppressing mixed blockers.
- Change the existing global and project `sync --yes` regressions to require
  exit 0 and `completed` after a healthy repair; retain exit 2 for
  unacknowledged, failed, or mixed-scope cases.
- Verify a repeat is a completed no-op and creates no additional backup in
  plain and JSON output.

## Acceptance

- Import postconditions fail closed on symlink, dangling, wrong-kind, and
  unreadable paths.
- Successful direct repair and target-scoped sync both complete after their
  exact filesystem postconditions hold.
- Disclosure output remains visible without keeping the result in attention.
- Mixed blockers and failed post-observation remain attention-required.

## Implementation notes

- Execution capability: strongest available; this closes a security-sensitive
  filesystem postcondition and a cross-command result aggregation contract.
- Review weight: standard, inherited from the autopilot caller.
- Files changed: `crates/cli/src/application/instructions.rs`,
  `crates/cli/src/application/reconciliation.rs`, and
  `crates/cli/tests/compiled_binary.rs`.
- Tests added: an isolated unit regression proving matching bytes behind a
  symlink fail the import postcondition; global and project target-scoped sync
  regressions now require exit 0/completed after acknowledged repair; direct
  repeat coverage verifies JSON and plain no-ops create no additional backup.
- Discrepancies from design: the existing sync regression used one Codex-shaped
  fake for both harnesses, which correctly produced an unrelated Claude
  capability warning. It now uses exact Codex and Claude fixtures so the test
  isolates repair disclosure semantics.
- Adjacent issues parked: none.

## Verification

- Focused import postcondition, direct repair/repeat, global/project sync, and
  broken duplicate regressions pass in isolated roots.
- Full workspace verification is recorded with the integration commit after
  the concurrent managed-load contract worker releases the shared files.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep review at the project-default `standard` weight,
escalated for the security-sensitive no-follow filesystem boundary. The
descriptor-bound import postcondition unit regression and the isolated global,
project, repeat, and broken-duplicate compiled scenarios pass at detached
commit `6c657f0`; exact postconditions gate completion.
