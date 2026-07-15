---
id: gate-tests-declaration-daemon-skip-matrix
kind: story
stage: done
tags: [testing]
parent: null
depends_on: []
release_binding: 3.1.0
gate_origin: tests
created: 2026-04-02
updated: 2026-07-15
---

# Byte-verify declaration-managed daemon skips across every target

## Priority
High

## Value evidence
Item: `epic-expanded-harness-support-declaration-managed`

The daemon contract requires declaration-managed install, update, and removal work to remain pending without changing target files, inventory, state bindings, or operation journals. Kiro has byte-level compiled-binary coverage; Kimi, Vibe, Kilo, Junie, and Amp do not.

## Gap type
important-interface / bug-regression

## Suggested test

Extend compiled-binary coverage for Kimi, Vibe, Kilo, Junie, and Amp. After an acknowledged install, snapshot the target home and project trees, run `daemon run --json`, assert exit code 2 with `attention_required` and pending operations, and assert byte-for-byte tree identity. Preserve the existing no-native-process expectation.

## Test location
`crates/cli/tests/compiled_binary.rs`

## Verification

Added an isolated compiled-binary matrix for Kimi, Vibe, Kilo, Junie, and Amp that snapshots managed roots, inventory, and resource state; proves daemon pending behavior; and permits only version probes.

- Focused core, application, and compiled-binary tests pass.
- `cargo test --workspace --all-targets`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- Independent standard review: no material findings.
- `cargo fmt --all -- --check` and `git diff --check`: clean.
