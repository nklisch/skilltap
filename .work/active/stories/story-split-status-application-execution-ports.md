---
id: story-split-status-application-execution-ports
kind: story
stage: done
tags: [refactor]
parent: feature-split-status-application
depends_on: []
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract StatusApplication execution ports

## Brief

Move the state-backed execution journal and the managed skill/instruction
execution ports from `crates/cli/src/application.rs` into the private module
`crates/cli/src/application/execution.rs`. Preserve the exact `ExecutionJournal`
and `ExecutionPort` behavior, evidence codes, filesystem ordering, and rollback
semantics.

## Current / target

Current code is the top-level block at `application.rs:101-630`:
`StateExecutionJournal`, `ManagedSkillPort`, `ManagedSkillEntry`,
`ManagedSkillAction`, `InstructionPort`, `InstructionEntry`,
`InstructionWrite`, and their trait implementations. Lifecycle and instruction
methods instantiate them directly.

Target `execution.rs` owns those types and implementations under
`pub(super)` visibility. `application.rs` declares `mod execution;` and imports
the types for sibling modules. No public method or entrypoint composition
changes.

## Acceptance criteria

- `application/execution.rs` contains the three port implementations and the
  parent contains no duplicate definitions.
- Operation-surface revalidation, managed-tree backup/replacement/removal,
  instruction bridge writes, and state journaling are byte/behavior compatible.
- `cargo test -p skilltap-cli --offline`, workspace fmt, and workspace clippy
  pass; existing tests and output assertions are unchanged.

## Risk / rollback

Private visibility and lifetime/import mistakes are the primary risk. Revert
the extraction commit to restore the blocks to `application.rs`; the move does
not touch persisted state or native files.

## Implementation Notes

- Moved `StateExecutionJournal`, `ManagedSkillPort`, and `InstructionPort`
  (including their entries, actions, validation, rollback, and failure helpers)
  into `crates/cli/src/application/execution.rs`.
- Kept the application façade's imports and all concrete construction sites
  behaviorally identical; only private module visibility and imports changed.
- Verification: `cargo fmt --all -- --check` and `cargo test -p skilltap --offline`
  passed (40 unit tests and 41 compiled-binary tests).

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard same-harness fresh-context review. The extraction is
mechanical with unchanged port logic, visibility limited to the application
module tree, and unchanged construction sites; workspace fmt, tests, clippy,
and diff checks are green.
