---
id: epic-rust-control-plane-runtime-primitives-errors-paths
kind: story
stage: done
tags: [infra]
parent: epic-rust-control-plane-runtime-primitives
depends_on: []
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
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

## Implementation notes

- Files changed: `crates/core/src/lib.rs` and new `crates/core/src/runtime/{mod,error,paths}.rs` modules.
- Public surface: category-preserving `RuntimeError` variants and context enums; `Environment` port with `ProcessEnvironment`; supported-platform detection; immutable `PlatformPaths` resolution for home, XDG config home, skilltap config, global `AGENTS.md`, Codex home, and Claude home.
- Tests added: 8 unit tests covering all seven runtime boundary categories, safe error rendering, XDG override and empty/absent fallback, missing/relative/noncanonical paths, non-UTF-8 environment values, unsupported platforms, required home-relative locations, and resolution without filesystem creation.
- Design details: invalid environment values are discarded before constructing errors, so display/debug output retains the variable and typed cause but not raw input; empty `XDG_CONFIG_HOME` follows `${XDG_CONFIG_HOME:-$HOME/.config}` semantics while empty `HOME` is missing.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch rationale: direct-read only; the runtime module did not exist and the story declared a bounded error/path surface.
- Verification: `cargo fmt --all -- --check`, `cargo check --locked --workspace --all-targets`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace`, and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` all pass (66 workspace tests).

## Review

Approved. The injected environment boundary keeps resolution deterministic and
side-effect-free; XDG fallback, strict path validation, supported-platform
classification, and all typed error categories match the story. Invalid raw
environment values are absent from both display and debug output. The focused
eight-test runtime suite and warnings-denied workspace Clippy pass on review.
