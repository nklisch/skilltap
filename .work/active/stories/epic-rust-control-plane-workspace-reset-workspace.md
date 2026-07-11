---
id: epic-rust-control-plane-workspace-reset-workspace
kind: story
stage: implementing
tags: [infra, cleanup]
parent: epic-rust-control-plane-workspace-reset
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Reset Repository and Bootstrap Rust Workspace

## Scope

Implement Unit 1 from the parent feature: remove the retired product and root
JavaScript toolchain, create the pinned four-crate Cargo workspace, generate
the lockfile, and provide a minimal version/help-capable `skilltap` binary.
Preserve the website, installer, Homebrew, workflows, foundation, research,
agent configuration, and work substrate for their owning stories.

## Acceptance criteria

- [ ] The four-crate workspace builds and tests on Rust 1.96.0 / Edition 2024.
- [ ] The release binary reports version 3.0.0 and renders help.
- [ ] Retired TypeScript product/tooling paths and committed legacy state are gone.
- [ ] Production crate dependency direction matches `docs/ARCH.md`.
