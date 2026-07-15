---
id: epic-expanded-harness-support-declaration-managed-acceptance
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-declaration-managed
depends_on: [epic-expanded-harness-support-declaration-managed-daemon-safety, epic-expanded-harness-support-declaration-managed-migration-regressions]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-15
gate_origin: null
created: 2026-07-15
updated: 2026-07-15
---

# Prove Declaration-Managed Acceptance End to End

## Checkpoint

Extend the existing adapter and managed-projection acceptance matrices to prove
the complete declaration-managed contract through production lifecycle dispatch,
status, rollback, and daemon policy in isolated environments.

## Design element

- Add dependency-neutral fixture descriptors for scoped Supported/Unverified/
  Unsupported capabilities and permitted complete-tree/managed-document
  declaration surfaces.
- Exercise real production-aware callbacks for:
  - Supported skill plus Unverified MCP;
  - all-Unverified managed projection;
  - Unsupported required and optional components;
  - managed port without declaration contract;
  - exact known and adjacent unknown versions.
- Cover explicit plugin/skill commands, `plan`, `sync`, status, update/remove,
  pending retry, rollback failure/residual output, immediate repeat, and daemon
  run in isolated HOME/XDG/project roots.
- Preserve current Codex/Claude and landed managed-adapter regression matrices.

## Acceptance evidence

- Supported remains ordinary/no-ack; declaration `Unverified` blocks without
  `--yes` and applies only in foreground with `--yes`; Unsupported always blocks.
- Native `Unverified` cannot be acknowledged.
- Scope and component results are independent; required dependency loss blocks
  while safe siblings remain visible.
- Plans expose exact files/complete-skill roots, selectors, reversibility, and
  material consequences.
- Status shows declared ownership/health separately from effective unverified
  and never derives loaded/healthy from file presence.
- Ambiguous/unmanaged collisions, malformed documents, literal secrets, drift,
  unknown versions, and races produce no write and preserve unrelated bytes.
- Accepted operations prove disk verification, ownership-safe update/removal,
  pending recovery, rollback/residual reporting, target isolation, and
  immediate-repeat no file/inode/plan/state change.
- Daemon skips declaration-managed work while applying independent Supported
  work.
- Unknown/adjacent versions never mutate through any command even with `--yes`.
- `cargo test --workspace --all-targets`, all-feature Clippy with warnings
  denied, formatting, and `git diff --check` pass.

## Implementation notes

- Production-aware compiled CLI coverage exercises exact profile routing,
  conditional target isolation, managed partial acknowledgment, foreign
  collisions, rollback-safe writes, unknown-version no-write behavior, status,
  sync, and daemon paths in isolated machine roots.
- Existing managed-adapter and standalone skill matrices remain green after the
  shared operation contract migration; the full workspace/all-target suite
  passes with `704` tests.

## Ordering constraint

Final checkpoint. It depends on daemon safety and migration/regression
completion and makes the parent feature eligible for feature-level review.
