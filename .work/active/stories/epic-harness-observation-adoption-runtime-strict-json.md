---
id: epic-harness-observation-adoption-runtime-strict-json
kind: story
stage: done
tags: [infra,correctness]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Decode Strict Bounded Native JSON

Implement a byte-capped UTF-8 boundary that accepts exactly one JSON document
with trailing whitespace only. Use a recursive seed/visitor to reject duplicate
object keys at every depth and enforce an explicit nesting limit before typed
decode; do not rely on `serde_json::Value` last-key-wins behavior. Reject
trailing documents/garbage and invalid UTF-8 with fixed safe errors that never
echo native bytes or parser excerpts. Honor the contract's hard stack-safe
depth ceiling and test zero plus every hard limit at minus/at/plus one.

## Implementation notes

- Files changed: `crates/core/src/runtime/strict_json.rs`,
  `crates/core/src/runtime/observation.rs`, `crates/core/src/runtime/mod.rs`.
- Added `StrictJson`, a pure implementation of the existing decoder port. It
  checks the byte cap before UTF-8 conversion, recursively builds one typed
  value through duplicate-aware sequence/map visitors, and accepts only
  trailing whitespace after the document.
- Container depth has one explicit meaning for both arrays and objects. The
  hard ceiling is 127 containers, the highest value accepted below
  serde_json's built-in recursion guard; the guard remains enabled as a second
  stack-safety boundary.
- Duplicate keys at root or any nested object, invalid UTF-8, invalid syntax,
  trailing documents/garbage, byte overflow, and depth overflow map only to
  fixed closed errors. No parser error text or input bytes cross the port.
- Tests cover contract zero/hard-max rejection, byte and nesting
  minus/at/plus-one behavior (including the hard depth ceiling), nested
  duplicates in objects/arrays, invalid UTF-8, trailing whitespace/document/
  garbage behavior, typed success, and secret canaries across Display/Debug/
  serde errors.
- Discrepancies from design: the provisional JSON hard ceiling moved from 128
  to 127 after executable boundary tests proved serde_json reserves the next
  recursion level. The parser guard was preserved rather than disabled.
- Adjacent issues parked: none.

## Verification

- `cargo test -p skilltap-core runtime::strict_json --locked`
- `cargo clippy -p skilltap-core --all-targets --locked -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo check --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- Full workspace tests reached 251 passing tests before the unrelated,
  concurrently edited test-support fixture
  `native_process::tests::executable_publication_survives_parallel_fixture_churn`
  failed with `ETXTBSY`; correction remains with that story's owner.

## Review

- Approved after fresh-context review plus an additional adversarial probe.
- Confirmed byte/UTF-8 precedence, escaped-equivalent and nested duplicate-key
  rejection, exactly-one-document behavior, scalar/numeric/composite fidelity,
  fixed source-free errors, and the 127/128 recursion boundary.
- Focused tests pass 5/5 and strict core Clippy passes; the only coordinated
  full-workspace exception is tracked in the fixture story.
