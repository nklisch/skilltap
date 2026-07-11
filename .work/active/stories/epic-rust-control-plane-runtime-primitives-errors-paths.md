---
id: epic-rust-control-plane-runtime-primitives-errors-paths
kind: story
stage: implementing
tags: [infra]
parent: epic-rust-control-plane-runtime-primitives
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Runtime Errors and Platform Paths

## Brief

Add the typed runtime boundary error model and deterministic Linux/macOS path
resolution used by all later runtime adapters.

## Acceptance criteria

- Runtime failures distinguish environment, path, filesystem, lock, command,
  clock, and unsupported-platform boundaries with safe structured context.
- Resolve `${XDG_CONFIG_HOME:-$HOME/.config}/skilltap`, `~/AGENTS.md`, and
  required home-relative locations without creating them.
- Reject missing/relative/non-UTF-8 inputs and return validated `AbsolutePath`
  values; reads have no process or terminal side effects.
- Unit tests cover XDG override/fallback, missing HOME, normalization, and safe
  error rendering.
- Locked formatting, all-target check, Clippy, tests, and rustdoc pass.

## Design notes

Use injected environment access in tests. Do not mutate global environment or
create the configuration directory during resolution.
