---
id: epic-rust-control-plane-runtime-primitives-git-probe-errors
kind: story
stage: implementing
tags: [infra, correctness]
parent: epic-rust-control-plane-runtime-primitives
depends_on: [epic-rust-control-plane-runtime-primitives-scope-target]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Classify Git Root Probe Failures

## Brief

Preserve non-repository project fallback while refusing to silently reinterpret
an existing broken, inaccessible, or rejected Git repository as a nested
standalone project.

## Acceptance criteria

- A directory with no containing Git metadata and an ordinary nonzero probe
  still returns `None` so project scope falls back to the canonical directory.
- When a nonzero probe occurs and `.git` metadata exists at the candidate or an
  ancestor, return a typed safe Git-root error containing directory/status but
  not stderr or environment content.
- Ancestor inspection is limited to the supplied canonical directory chain; it
  does not discover projects or resources elsewhere.
- Tests cover ordinary non-Git fallback, corrupt `.git` file/directory, a
  rejected probe fixture, nested metadata, and safe error rendering.
- Full locked format/check/Clippy/test/rustdoc ladder passes.
