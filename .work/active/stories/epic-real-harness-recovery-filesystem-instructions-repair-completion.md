---
id: epic-real-harness-recovery-filesystem-instructions-repair-completion
kind: story
stage: implementing
tags: [correctness, security, testing]
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
