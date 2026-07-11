---
id: epic-cross-harness-materialization-hooks-contract
kind: story
stage: done
tags: []
parent: epic-cross-harness-materialization-hooks
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Normalize Hook Contracts

Add validated hook contract types in core and bounded Codex/Claude readers in
the harness adapters.

Acceptance criteria:

- Event, payload, failure, cwd, environment references, and executable
  permission semantics are normalized without raw secret values.
- Malformed or unsafe hook declarations fail before mapping.
- Reader tests cover both harness fixtures and observation-only behavior.

## Implementation notes

- Files changed: `crates/core/src/hook_mapping.rs`, `crates/core/src/lib.rs`.
- Tests added: faithful contract and required/optional payload mismatch
  classification tests.
- Discrepancies from design: adapter readers remain the next child story; this
  stride establishes the closed normalized contract and evidence constructor
  without inventing native event schemas.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core hook_mapping::tests --offline` — passed.
- `cargo clippy -p skilltap-core --all-targets --offline -- -D warnings` —
  passed.

## Review

Verdict: Approve — story verified by implement; fast-lane advance.
