---
id: epic-real-harness-recovery-native-lifecycle-scope-aware-presence
kind: story
stage: done
tags: [correctness, testing]
parent: epic-real-harness-recovery-native-lifecycle
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Match native resource presence by concrete scope

## Finding

Claude list commands omit `--scope` and return entries carrying their own
scope. The current parser compares only identity, so an equal global resource
can be mistaken for the requested project/local resource during journal
re-observation.

## Required fix

- Project list observations by both requested identity and the exact native
  scope represented by the request; global maps to `user` and personal project
  scope maps to the attested `local` value.
- Treat missing, malformed, duplicate, or contradictory scope evidence as
  unknown rather than borrowing evidence from another scope.
- Preserve Codex behavior only where its attested list shape is genuinely
  scope-unambiguous.
- Add adapter and compiled lifecycle regressions where a same-name global
  resource coexists with a missing/drifted project resource and cannot satisfy
  the project re-observation.

## Acceptance

- Native presence evidence never crosses a concrete global/project boundary.
- A removed project resource is reapplied even when its global sibling exists.
- Malformed scope evidence fails closed without exposing raw payloads.

## Implementation notes

- Execution capability: strongest available; this is a correctness boundary
  that decides whether a previously applied native mutation may be skipped.
- Review weight: highest, inherited from the recovery/autopilot run.
- Files changed: `crates/harnesses/src/lifecycle.rs`,
  `crates/harnesses/tests/lifecycle_scope.rs`,
  `crates/harnesses/tests/bootstrap.rs`, and
  `crates/cli/tests/compiled_binary.rs`.
- Tests added: exact global/project sibling matching, user/local coexistence,
  missing and malformed scope, duplicate same-scope entries, contradictory
  identity fields, isolated native subprocess observation, and a compiled CLI
  journal-repair scenario where the same-name user plugin remains present
  after the local plugin is removed.
- Discrepancies from design: none. Codex retains identity-only matching because
  its verified global lifecycle request is scope-unambiguous; Claude requires
  an exact `user` or `local` scope on every list entry before absence is
  authoritative.
- Verification: `cargo test -p skilltap-harnesses --all-targets` passes (49
  tests). The focused compiled CLI regression is temporarily blocked by the
  concurrent per-target state migration's unfinished CLI call sites and is
  ready to run when that shared transition compiles.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh-context deep lane at review weight `standard` (explicit
caller selection). Adapter, isolated subprocess, and compiled CLI regressions
prove that `user` evidence cannot satisfy `local`, same-name siblings remain
distinct, and missing/malformed/duplicate scope evidence fails closed. The
full workspace passes 519 tests. Correctness, tests, contract behavior,
failure handling, data exposure, and foundation alignment were reviewed.
