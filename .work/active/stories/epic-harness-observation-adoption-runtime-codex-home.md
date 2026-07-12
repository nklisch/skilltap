---
id: epic-harness-observation-adoption-runtime-codex-home
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits]
release_binding: 3.0.0
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Resolve Codex Home Without Moving Global Instructions

Extend runtime path resolution so a non-empty normalized absolute `CODEX_HOME`
wins and absent/empty falls back to `$HOME/.codex`. Reject relative,
non-normalized, and non-UTF-8 values without rendering bytes.
Resolution creates nothing. XDG continues to relocate only skilltap state and
Codex-native paths never relocate canonical global `~/AGENTS.md`.

## Implementation notes

- Files changed: `crates/core/src/runtime/error.rs`,
  `crates/core/src/runtime/paths.rs`.
- Tests added: focused path-policy coverage for `CODEX_HOME` override and
  absent/empty fallback, XDG and global-instruction independence, invalid and
  non-UTF-8 rejection, and no filesystem creation.
- Discrepancies from design: no filesystem accessibility check is performed;
  path policy intentionally resolves normalized absolute paths without
  requiring them to exist.
- Adjacent issues parked: none.

## Review

- Initial review approved runtime behavior but found foundation/public docs
  still hard-coded `~/.codex`; correction `2718009` aligned native paths with
  `${CODEX_HOME:-$HOME/.codex}` without moving skilltap state or `~/AGENTS.md`.
- Re-review found no remaining drift. Nine focused path tests and the website
  build pass; repeated LLM-document generation is byte-identical.
- The story requirement now explicitly reflects the no-I/O contract: policy
  resolution validates path shape but does not require the path to exist.
