---
id: story-split-status-application-instructions
kind: story
stage: done
tags: [refactor]
parent: feature-split-status-application
depends_on: [story-split-status-application-execution-ports]
release_binding: 3.0.0
gate_origin: refactor-design
created: 2026-07-12
updated: 2026-07-12
---

# Extract instruction bridge management

## Brief

Move instruction status, setup/repair, reconciliation preview, bridge path and
health helpers into `crates/cli/src/application/instructions.rs` after the
execution-port extraction. Preserve canonical `AGENTS.md`, Claude symlink or
import mode, nested bridge detection/consolidation, backups, and acknowledgment
semantics exactly.

## Current / target

Current methods are `execute_instruction_status`,
`execute_instruction_reconciliation_preview`, `execute_instruction_setup`, and
`execute_instruction_setup_for_target` (roughly `application.rs:918-3780`),
with bridge helpers at `application.rs:4404-4585`.

Target `instructions.rs` owns those methods in an `impl StatusApplication<'_>`
block with unchanged `pub(crate)`/`pub(super)` signatures and owns instruction
location, resource/operation ID, bridge health, preferred-path, desired-resource,
and backup-path helpers. `InstructionPort` and `InstructionWrite` are imported
from `execution.rs`.

## Acceptance criteria

- Global and project status/setup/repair preserve path selection, output order,
  duplicate nested Claude handling, backup-before-remove behavior, symlink vs
  import contents, and `--yes` repair behavior.
- Plan preview and sync setup retain operation IDs, paths, warnings, and result
  classes; no filesystem safety boundary changes.
- Instruction tests, workspace fmt, tests, and clippy pass.

## Risk / rollback

Relative project bridge paths and backup ordering are sensitive to accidental
context changes. Revert the extraction commit; no native or state migration is
needed.

## Implementation Notes

- Moved instruction status, reconciliation preview, setup, and target-specific
  bridge management into private `application/instructions.rs` methods.
- Preserved canonical/bridge path resolution, duplicate consolidation, backup
  behavior, symlink/import writes, operation IDs, and acknowledgment handling;
  sibling reconciliation calls use `pub(super)` visibility.
- Verification: `cargo fmt --all`, `cargo check -p skilltap --offline`, and
  `cargo test -p skilltap --offline` passed (40 unit tests and 41 compiled-
  binary tests).

## Review (2026-07-12)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: none
**Nits**: The bridge helper functions remain in `application.rs` as shared
private support rather than moving into `instructions.rs`; this does not alter
the private module boundary or behavior.

**Notes**: Standard same-harness fresh-context review. Instruction entrypoints
and execution-port calls preserve path selection, ordering, duplicate handling,
backup/write semantics, and acknowledgment behavior; workspace fmt, tests,
clippy, and diff checks are green.
