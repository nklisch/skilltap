---
id: epic-rust-control-plane-storage-removal-residuals
kind: story
stage: done
tags: [correctness]
parent: epic-rust-control-plane-storage
depends_on: [epic-rust-control-plane-storage-managed-artifacts]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Report Managed Removal Residuals

Extend runtime and storage errors for partial tree removal with expected and
observed identity, path presence, content progress (`intact`, `partial`,
`empty`, or `unknown`), and parent-directory sync state. Recursive deletion must
track whether any owned entry was removed; top unlink and parent sync are
reported independently. Add injected tests for failure before/after partial
content removal, identity/path replacement, empty-but-present, and
unlink-success/sync-failure. State/reference callers must be able to choose
re-observation without guessing. Preserve safe error rendering and run the full
locked ladder.

## Implementation Notes

- Added a structured runtime removal residual carrying the expected and
  observed identities, path presence, recursive content progress, and parent
  sync state.
- Made recursive deletion propagate whether it removed any owned entry and
  whether the opened directory reached empty, without changing publication
  cleanup behavior.
- Split top-directory unlink from parent-directory sync so an empty-present
  destination and a removed-but-not-proven-durable destination remain distinct.
- Mapped runtime residuals into a dedicated storage `PartialRemoval` failure and
  `ManagedRemovalResidual`; publication residual access remains unchanged.
- Covered pre-change failure, partial deletion, replacement, injected top
  unlink failure, injected parent sync failure, and storage mapping.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace` (150 tests)
- `cargo build --locked --release -p skilltap`
- 30 concurrent locked core unit-suite runs across eight workers

## Review

Approved. The runtime residual preserves expected and observed identity, exact
path presence, recursive content progress, and parent-sync durability without
following or removing a replacement directory. Storage exposes the recovery
state through a dedicated safe residual. Focused runtime and storage suites,
formatting, and strict locked Clippy all pass.
