---
id: gate-tests-native-journal-after-apply-recovery
kind: story
stage: implementing
tags: [testing]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: tests
created: 2026-04-02
updated: 2026-07-15
---

# Cover native journal-after-apply inventory recovery

## Priority
High

## Value evidence
Items: `feature-daemon-marketplace-refresh`, `epic-expanded-harness-support-declaration-managed`

When a native lifecycle action succeeds but the following journal write fails, the CLI must return attention-required and recover durable inventory to reflect the target state that was actually applied. Core coverage proves the executor marks the boundary as after-apply; no CLI-level test proves inventory recovery and publication.

## Gap type
bug-regression / important-interface

## Suggested test

Inject a state repository whose post-apply journal write fails after a fake native install succeeds. Assert attention-required, the journal-boundary error, native presence, and inventory containing only the successfully applied target binding. Cover removal symmetrically or prove the shared projection branch with a focused application-level test.

## Test location
`crates/cli/src/application/tests.rs` or `crates/cli/tests/compiled_binary.rs`
