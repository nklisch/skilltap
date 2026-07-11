---
id: epic-rust-control-plane-cli-shell-composition
kind: story
stage: review
tags: [cli, infra]
parent: epic-rust-control-plane-cli-shell
depends_on: [epic-rust-control-plane-cli-shell-command-model, epic-rust-control-plane-cli-shell-output]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Compose the Foundation CLI

Compose system runtime and typed storage adapters behind command dispatch.
Implement read-only first-use `status` with real scope/target/storage
validation and explicit native-observation attention. Route all later-capability
commands to stable pre-mutation unavailable outcomes. Keep handlers free of
native-format/domain business logic and run the locked ladder.

## Implementation notes

- Files changed: `crates/cli/src/application.rs`,
  `crates/cli/src/application/tests.rs`, `crates/cli/src/dispatch.rs`,
  `crates/cli/src/entrypoint.rs`, `crates/cli/src/entrypoint/tests.rs`,
  `crates/cli/src/lib.rs`, and `crates/cli/src/main.rs`.
- Composed `PlatformPaths`, the system filesystem and command runner,
  command-backed Git-root resolution, scope resolution, and the typed config,
  inventory, and state repositories at the binary boundary.
- Implemented read-only first-use `status`: all owned documents load and
  validate independently, missing config uses explicit in-memory defaults,
  missing inventory makes all-scopes global-only, relative project arguments
  resolve against the working directory before entering the absolute-path
  domain, and enabled targets resolve through the core target contract.
- Status never creates storage or claims native health. Successful foundation
  inspection returns attention with the stable
  `native_observation_unavailable` warning; zero enabled harnesses and disabled
  explicit targets return safe actionable outcomes.
- Exhaustively dispatched every other valid command to a stable
  `capability_unavailable` outcome before system composition or mutation.
- Normalized Clap failures into the skilltap outcome contract, including a
  single compact JSON document when `--json` is present, while help and version
  retain their successful Clap documents. No parser, storage, runtime, native,
  source, or debug values cross the safe error boundary.
- Tests added: ten focused application/entrypoint tests covering first use,
  no-write behavior, all-scopes without inventory, relative project paths,
  empty/disabled targets, independent malformed documents, unavailable
  commands, normalized JSON parse failure, missing commands, help, and version.
- Verification passed: `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --workspace` (182 tests),
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`,
  `cargo build --locked --release -p skilltap`, and compiled-binary smoke for
  help, first-use status, JSON parse errors, unavailable commands, exit codes,
  output channels, and no-created configuration root.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Dispatch rationale: one bounded composition surface with approved ports;
  direct implementation only, with no exploratory fanout.
