---
id: epic-rust-control-plane-workspace-reset
kind: feature
stage: drafting
tags: [infra, cleanup]
parent: epic-rust-control-plane
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Rust Workspace Reset

## Brief

Replace the retired TypeScript, Bun, and npm product implementation with a
buildable Rust Edition 2024 workspace containing the four architectural crates.
Pin the stable toolchain, establish dependency direction and baseline quality
checks, and provide the test-support skeleton needed by subsequent features.

The cleanup also rewrites build, CI, release, installer, and Homebrew plumbing
where it assumes the old runtime. The website, Homebrew distribution, installer,
and release experience remain product surfaces; this feature does not redesign
their content or implement v3 resource-management behavior.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: foundation feature — every other control-plane feature
  depends on the clean workspace and crate graph it establishes
- Inherits the clean-reset and pinned-stable decisions recorded by the parent

## Foundation references

- `docs/ARCH.md` — Workspace, Dependency Direction, Testing, Technology
- `docs/VISION.md` — Non-Goals, Principles
- `AGENTS.md` — Architecture, Development

<!-- The feature design pass will fill in implementation units. -->
