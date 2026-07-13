---
id: story-fix-managed-skill-rollback-residuals
kind: story
stage: done
tags: [bug]
parent: null
depends_on: []
release_binding: 3.0.2
research_refs: []
research_origin: null
gate_origin: patterns
created: 2026-07-12
updated: 2026-07-12
---

# Report managed skill rollback residuals

## Symptom

When replacement publication fails, the managed skill executor discards the
result of restoring the backed-up tree and reports only the original publish
failure. An agent cannot tell whether the prior skill was restored or whether
the managed destination now needs recovery.

## Root cause

`ManagedSkillPort::apply` invokes `publish_tree_no_follow` for the backup with
`let _ =`, losing both the restore result and any residual evidence.

## Fix approach

Require restoration to publish the exact prior tree, verify its identity or
contents, and distinguish proven restoration from a residual/uncertain managed
destination in the typed failure detail.

## Regression test

Add a fault-injected managed-skill replacement test that fails new publication
and backup restoration, then asserts the destination is never described as
restored and the recovery surface is named. Retain a clean-restoration case.

## Implementation

- Replacement rollback now re-reads the restored directory without following
  links, verifies the published identity when available, and compares the full
  restored artifact tree with the durable backup.
- Failure evidence distinguishes a proven restoration from an uncertain
  residual and names the exact managed destination in both cases.
- The fault filesystem can schedule multiple tree-publication failures, which
  covers both the replacement publish and the subsequent restore attempt.

## Verification

- `cargo test -p skilltap managed_skill_replacement_reports_clean_and_uncertain_restoration -- --nocapture`
- `cargo test -p skilltap`
- `cargo clippy -p skilltap --all-targets -- -D warnings`

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none
**Rejected**: none

**Notes**: Substrate Deep review at effective `standard` weight (explicit
caller selection), performed in same-harness fresh context over commit
`2b65680`. Replacement failure restores only through the no-follow tree port,
captures and rechecks the identity returned by a successful publication, and
compares the complete restored artifact tree with the durable backup. Both
clean and failed restore paths name the exact managed destination; the failed
path is conservatively described as unproven and never as restored. The fault
fixture schedules the replacement and restore failures independently while
delegating successful publication/load behavior to the real filesystem
adapter, so the regression is behavioral rather than tautological. The focused
clean/uncertain regression passes. Correctness, rollback/race safety,
diagnostic truthfulness, and test-integrity lenses found no remaining issue;
the change does not alter a public schema, CLI, or foundation assertion.
