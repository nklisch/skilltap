---
id: epic-rust-control-plane-workspace-reset-workspace
kind: story
stage: done
tags: [infra, cleanup]
parent: epic-rust-control-plane-workspace-reset
depends_on: []
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Reset Repository and Bootstrap Rust Workspace

## Scope

Implement Unit 1 from the parent feature: remove the retired product and root
JavaScript toolchain, create the pinned four-crate Cargo workspace, generate
the lockfile, and provide a minimal version/help-capable `skilltap` binary.
Preserve the website, installer, Homebrew, workflows, foundation, research,
agent configuration, and work substrate for their owning stories.

## Acceptance criteria

- [x] The four-crate workspace builds and tests on Rust 1.96.0 / Edition 2024.
- [x] The release binary reports version 3.0.0 and renders help.
- [x] Retired TypeScript product/tooling paths and committed legacy state are gone.
- [x] Production crate dependency direction matches `docs/ARCH.md`.

## Implementation notes

- Files changed: added the root Cargo workspace, pinned toolchain, lockfile, and
  four crate skeletons; updated `.gitignore`; removed the retired root
  JavaScript/Bun/TypeScript tooling, `packages/`, patches, product build script,
  legacy committed `.agents` state, and stale Claude-only rules.
- Tests added: none — this story establishes compile-only crate skeletons and a
  clap-generated help/version surface.
- Verification: `cargo fmt --all -- --check`, `cargo check --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo build --workspace`, and
  `cargo build --release -p skilltap` pass on Rust 1.96.0. The compiled binary
  reports `skilltap 3.0.0` and renders help. Cargo metadata reports exactly four
  workspace packages with CLI → core/harnesses and harnesses → core dependency
  direction.
- Discrepancies from design: the Cargo package at `crates/cli` is named
  `skilltap`, rather than the architectural label `skilltap-cli`, so the exact
  documented release command `cargo build --release -p skilltap` succeeds. The
  binary and crate boundary remain unchanged.
- Adjacent issues parked: none.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Story verified by implement; fast-lane advance.
