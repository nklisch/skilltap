---
id: epic-harness-observation-adoption-contracts-findings
kind: story
stage: review
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-graph]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Make Observation Findings Safe by Construction

Replace fixed coarse finding kinds plus arbitrary messages/JSON metadata with
validated open codes, authored static summaries, severity, typed subjects, and
a bounded scalar field vocabulary. Add secret canaries proving raw argv,
stdout/stderr, settings, unknown JSON, and dynamic messages cannot enter domain
findings or their Debug/Display/serde forms.

## Implementation notes

- Files changed: `crates/core/src/domain/resource/finding.rs`,
  `crates/core/src/domain/resource.rs`,
  `crates/core/src/domain/resource/layered_tests.rs`, and
  `crates/core/src/domain/mod.rs`.
- Replaced coarse finding kinds, dynamic messages, and arbitrary JSON metadata
  with validated open finding/field codes, exact authored static summaries,
  severity, harness-or-resource subjects, and a maximum of 32 typed scalar
  fields.
- Allowed field values are limited to booleans, counts, harness IDs, exact
  resource keys, resource kinds, capability IDs, and observation layers. No
  string, byte, path, collection, or arbitrary JSON value variant exists.
- Findings now have a total typed order, so resource graph output remains
  deterministic without canonicalizing arbitrary JSON.
- Tests added: authored round-trip/order, raw payload ingress rejection,
  Debug/Display/serde secret canaries, field-count and owned-wire strictness,
  and validated open-code behavior.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace` (216 tests across workspace suites)
- `cargo doc --locked --workspace --no-deps`
- `cargo build --locked --release -p skilltap`
- `scripts/verify-compiled-binary.sh /storage/cargo-target/release/skilltap`
