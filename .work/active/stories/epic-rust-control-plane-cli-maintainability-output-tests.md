---
id: epic-rust-control-plane-cli-maintainability-output-tests
kind: story
stage: review
tags: [refactor, testing]
parent: epic-rust-control-plane-cli-maintainability
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Move Output Tests to a Sidecar

Move the unchanged inline output test module to `output/tests.rs`, preserving
all fully qualified test identities, bodies, assertions, and test-list order.
Run the full locked ladder.

## Implementation notes

- Files changed: `crates/cli/src/output.rs` and
  `crates/cli/src/output/tests.rs`.
- Tests added: none; the seven output tests moved unchanged to the private
  sidecar.
- Test identity verification: the complete 34-test CLI library listing before
  and after the move is byte-identical, including fully qualified identities
  and list order. A normalized source comparison also confirms that every test
  body and assertion is unchanged.
- Verification: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (191 tests), and
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` all
  pass.
- Discrepancies from design: none.
- Adjacent issues parked: none.
