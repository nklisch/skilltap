---
id: epic-harness-observation-adoption-contracts-installation-profiles
kind: story
stage: done
tags: [infra]
parent: epic-harness-observation-adoption-contracts
depends_on: [epic-harness-observation-adoption-contracts-resource-key]
release_binding: null
research_refs: [.research/analysis/briefs/current-agent-extension-standards.md]
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Define Installation and Scoped Profile Contracts

Add resolved executable identity, configured binary, reachability, opaque
native version, compiled profile ID/authority, and global/project capability
sets. Enforce that unknown versions have no verified profile or mutation
authority and that probe results can only narrow existing support.

## Implementation notes

- Files changed: `crates/core/src/domain/installation.rs`,
  `crates/core/src/domain/mod.rs`.
- Tests added: seven unit contracts covering configured binary forms, exact
  executable identity, unreachable evidence, scope-varying capabilities,
  unknown-version observe-only authority, monotonic narrowing, and rejection
  of capabilities absent from the compiled profile.
- Discrepancies from design: none. The selected-profile enum makes verified
  compiled authority structurally distinct from unknown-version observation;
  only the verified variant exposes mutation capabilities.
- Adjacent issues parked: none.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace`
- `cargo doc --locked --workspace --no-deps`
- `cargo build --locked --release -p skilltap`
- `scripts/verify-compiled-binary.sh /storage/cargo-target/release/skilltap`

## Review

Approved. Reachability binds opaque version evidence to one exact executable;
capabilities vary by scope, narrowing is monotonic, and unknown versions are
structurally observe-only with no mutation-capability accessor.
