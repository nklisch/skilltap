---
id: epic-harness-observation-adoption-status-integration
kind: story
stage: done
tags: [cli,testing]
parent: epic-harness-observation-adoption-status
depends_on: [epic-harness-observation-adoption-status-policy, epic-harness-observation-adoption-status-observation]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify Harness Status CLI

Exercise plain and JSON list/enable/disable/status flows across global,
current/explicit project, all-scopes, and named targets. Cover first-use
no-create, partial sibling success, stable exit classes, repeat idempotence,
safe diagnostics, and native byte/type/link/mtime no-mutation.

## Implementation notes

- Extended the compiled CLI integration suite with first-use `harness list`,
  JSON/plain policy changes, binary overrides, and repeat-enable byte/mtime
  idempotence checks.
- Added a Codex-success/Claude-failure status fixture to verify sibling
  observations remain visible while the aggregate stays attention-required.
- Snapshotted native file bytes, entry types, symlink targets, and mtimes before
  and after status to assert the observation path is read-only.
- Isolated-machine subprocesses now clear an inherited `CODEX_HOME` so status
  fixtures cannot accidentally observe the developer's native tree.

## Verification

- `cargo test -p skilltap --test compiled_binary --offline` (8 passed).

## Review

Verdict: Approve - story verified by implement; fast-lane advance.
