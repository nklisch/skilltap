---
id: epic-rust-control-plane-workspace-reset-ci
kind: story
stage: implementing
tags: [infra]
parent: epic-rust-control-plane-workspace-reset
depends_on: [epic-rust-control-plane-workspace-reset-workspace]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Establish Rust CI and Binary Verification

## Scope

Implement Unit 2 from the parent feature: replace product CI with Rust format,
clippy, test, release-build, and compiled-binary smoke gates. Rewrite the smoke
script for the Rust artifact and keep it extensible for later CLI commands.

## Acceptance criteria

- [ ] Every CI command passes locally.
- [ ] CI has no Bun/npm product dependency.
- [ ] Binary verification checks `--version` and `--help` and fails clearly for
  a missing or broken executable.
