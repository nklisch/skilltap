---
id: epic-rust-control-plane-cli-shell-integration
kind: story
stage: done
tags: [cli, testing]
parent: epic-rust-control-plane-cli-shell
depends_on: [epic-rust-control-plane-cli-shell-composition]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Verify the Compiled CLI Contract

Add compiled-binary integration coverage for the full grammar, no-subcommand,
help/version, first-use no-create status, project/all-scope and target forms,
malformed storage, unavailable handlers, one-document JSON, safe plain output,
and exit codes. Run the full locked ladder plus release-binary smoke checks.

## Implementation notes

- Files changed: `crates/cli/tests/compiled_binary.rs`, `crates/cli/Cargo.toml`,
  `Cargo.lock`, `.github/workflows/ci.yml`, and `.github/workflows/release.yml`.
- Added six isolated compiled-binary integration tests covering every command
  leaf; root and nested help; version; bare invocation; first-use read-only
  status; global, current-project, explicit-project, and all scopes; omitted,
  named, and all targets; malformed config, inventory, and state; unavailable
  handlers; parser failures; JSON document purity; plain output channels; and
  exit codes 0, 1, and 2. Exit code 3 remains renderer-level coverage because
  no foundation command can truthfully produce a partial apply.
- CI and every release-platform build now rerun the same integration suite with
  `SKILLTAP_TEST_BIN` pointing at the optimized release binary before artifact
  publication.
- The bare-invocation assertion initially exposed the separately tracked
  `epic-rust-control-plane-cli-shell-bare-help` defect; its approved correction
  at `db95263` now passes through the compiled boundary.
- Verification: locked format, check, Clippy with warnings denied, 191 workspace
  tests, rustdoc, optimized release build, six tests against the actual release
  binary, and `scripts/verify-binary.sh` all pass.
- Discrepancies from design: none.
- Adjacent issues parked: `idea-cli-bare-invocation-omits-help` was promoted and
  resolved by the sibling bare-help story.

## Review

Approved. The isolated compiled-binary suite covers every leaf command and the
documented scope, target, storage, channel, JSON, help/version, no-create, and
0/1/2 exit contracts; partial exit `3` remains truthfully covered at the
renderer boundary. CI and each native release runner exercise the optimized
binary before publication.
