---
id: epic-rust-control-plane-macos-ci
kind: story
stage: done
tags: [infra, testing]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-cli-shell]
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: tests
created: 2026-07-11
updated: 2026-07-12
---

# Run Native macOS Contracts Before Merge

Add a native macOS pre-merge CI job that installs the pinned toolchain and runs
the locked workspace tests, optimized `skilltap` build, and explicit compiled
binary verification wrapper. Ensure Apple-specific unsafe filesystem, errno,
locking, managed-artifact, and recovery tests run on every push and pull
request. Keep release-matrix verification unchanged and avoid duplicating
website work.

## Implementation notes

- Files changed: `.github/workflows/ci.yml` and this story.
- Tests added: a native `macos-14` pre-merge job runs the locked full workspace
  tests, optimized `skilltap` build, and compiled-binary verification wrapper.
- Discrepancies from design: none. The job follows the existing pinned-toolchain
  and release-runner conventions while leaving Ubuntu quality, website, and the
  release workflow unchanged.
- Dispatch: direct-read implementation; the integration surface was limited to
  the existing CI and release workflow conventions.
- Adjacent issues parked: none.

## Review

Approved. Native macOS now runs the full locked workspace, optimized build, and
exact compiled-binary contract on every push and pull request while existing
Ubuntu, website, and release gates remain unchanged.
