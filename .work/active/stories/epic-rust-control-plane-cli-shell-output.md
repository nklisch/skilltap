---
id: epic-rust-control-plane-cli-shell-output
kind: story
stage: review
tags: [cli]
parent: epic-rust-control-plane-cli-shell
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Implement Stable Outcomes and Rendering

Implement the schema-1 outcome envelope, safe error/next-action values,
plain/JSON renderers, and the completed/invalid/attention/partial exit mapping.
Both representations must derive from one outcome, JSON must be exactly one
document, and renderers must not expose debug/native source values. Add focused
contract tests and run the locked ladder.

## Implementation notes

- Added the `skilltap` library surface with a schema-1 `Outcome`, four stable
  `ResultClass` variants, optional typed scope, deterministic scalar summaries,
  typed resource/operation entries, warnings, safe errors, and next actions.
- Added compact single-document JSON and concise plain renderers over the same
  outcome. JSON contains every required collection, omits only an absent scope,
  and emits no incidental text; plain output neutralizes terminal controls and
  line injection.
- Kept nested error sources and debug/native values out of the serializable
  error contract. Render failures expose a safe display message rather than the
  serializer detail.
- Centralized the completed/invalid/attention-required/partial-apply mapping as
  exit codes `0`/`1`/`2`/`3` derived solely from `ResultClass`.
- Added seven contract tests covering exact empty JSON, representative envelope
  fields, shared semantic rendering, scalar JSON types, safe error shape,
  terminal-control neutralization, and all exit classes.
- Added the CLI crate's direct `serde` and `serde_json` dependencies and refreshed
  its existing lockfile package entry offline; no dependency versions changed.
- Discrepancies from design: none. Adjacent issues parked: none.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (157 tests),
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, and
  `cargo build --locked --release -p skilltap`.
