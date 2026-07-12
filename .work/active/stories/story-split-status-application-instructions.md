---
id: story-split-status-application-instructions
kind: story
stage: implementing
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

