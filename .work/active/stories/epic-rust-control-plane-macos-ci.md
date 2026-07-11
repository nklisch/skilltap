---
id: epic-rust-control-plane-macos-ci
kind: story
stage: implementing
tags: [infra, testing]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-cli-shell]
release_binding: null
research_refs: []
research_origin: null
gate_origin: tests
created: 2026-07-11
updated: 2026-07-11
---

# Run Native macOS Contracts Before Merge

Add a native macOS pre-merge CI job that installs the pinned toolchain and runs
the locked workspace tests, optimized `skilltap` build, and explicit compiled
binary verification wrapper. Ensure Apple-specific unsafe filesystem, errno,
locking, managed-artifact, and recovery tests run on every push and pull
request. Keep release-matrix verification unchanged and avoid duplicating
website work.
