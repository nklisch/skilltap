---
id: story-skilltap-plugin-distribution-bootstrap-contract
kind: story
stage: review
tags: [infra, security]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [epic-skilltap-plugin-distribution-package]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Bootstrap release and update policy contract

Implement the pure bootstrap domain contract described by the parent feature.
Model supported release platforms and architectures, validated semver release
identity, artifact locators/checksums, binary update decisions, and the config
policy governing unattended binary updates.

Scope:

- `crates/core/src/bootstrap.rs` and module export.
- `crates/core/src/storage/config.rs` policy additions and schema fixtures.
- Pure unit tests under `crates/core/src/bootstrap/tests.rs`.

Acceptance criteria:

- Numeric release comparison returns install, same-major update, no-op, or
  major-upgrade-blocked according to the explicit `allow_major` decision.
- Fresh installs resolve the latest release; unattended updates default to
  safe same-major application and support `off`, `check`, and `apply-safe`.
- Unsupported platforms/architectures, malformed semver, controls, traversal,
  option-like values, and malformed checksums fail at construction.
- Config defaults and round trips preserve existing strict schema behavior and
  do not persist defaults from read-only commands.
- Tests remain pure and do not use network, filesystem, native processes, or
  the operator's state.

Do not add CLI parsing, HTTP, shell invocation, harness lifecycle calls, or
terminal output in this story. Record any required schema/version deviation in
this item before implementation.

## Implementation notes
- Execution capability: highest available local capability; release and update policy are security-sensitive contracts.
- Review weight: standard (source: autopilot project default).
- Files changed: `crates/core/src/bootstrap.rs`, `crates/core/src/lib.rs`, `crates/core/src/runtime/paths.rs`, `crates/core/src/storage/config.rs`.
- Tests added: pure release/version, artifact validation, decision-policy, and bootstrap config round-trip tests.
- Discrepancies from design: the binary policy is an optional `[bootstrap]` config table with deterministic defaults, preserving legacy config serialization when defaults are unchanged.
- Adjacent issues parked: none.
