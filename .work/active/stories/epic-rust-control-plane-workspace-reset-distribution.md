---
id: epic-rust-control-plane-workspace-reset-distribution
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

# Preserve Release, Installer, and Homebrew Distribution

## Scope

Implement Unit 3 from the parent feature: build and attest four native Rust
release artifacts, remove npm publication, and align the curl installer,
local installer, Homebrew formula, and formula-update automation with the
unchanged artifact naming contract.

## Acceptance criteria

- [ ] Release workflow builds and verifies Linux/macOS x64/arm64 binaries.
- [ ] Installer and formula use the same four asset names as the workflow.
- [ ] Shell scripts pass syntax checks and preserve configurable install paths.
- [ ] No release job publishes or requires the retired npm packages.
