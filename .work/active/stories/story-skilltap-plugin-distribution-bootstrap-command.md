---
id: story-skilltap-plugin-distribution-bootstrap-command
kind: story
stage: review
tags: [infra, content, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-artifacts, story-skilltap-plugin-distribution-bootstrap-harness]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# First-class bootstrap command and result contract

Expose the bootstrap application through the Rust CLI and compose the pure
release policy, artifact installer, and harness setup adapters. Add the
non-interactive `skilltap bootstrap` leaf with `--target`, `--allow-major`, and
`--json`; the command must report binary state and each target's plugin state
separately.

Scope:

- `crates/cli/src/command.rs`, `dispatch.rs`, `entrypoint.rs`.
- `crates/cli/src/application/bootstrap.rs` and composition exports.
- Command/entrypoint tests and compiled-binary coverage.

Acceptance criteria:

- Help describes global scope, target narrowing, major-version acknowledgment,
  JSON representation, no prompts, and exit classes.
- Plain and schema-1 JSON output distinguish install/update/no-op from each
  harness result and include safe next actions for missing/unsupported targets.
- Fresh install takes latest release; existing major upgrades require
  `--allow-major`; same-major repeats are idempotent.
- Failures before verified publish leave binary/config/native state unchanged;
  no `--yes` or arbitrary source argument is introduced.
- Compiled tests cover target narrowing, absent harnesses, mixed success and
  attention, blocked major upgrade, and healthy repeat in isolated roots.

Keep command grammar authoritative in clap metadata and preserve existing
output schema/result classes. Do not duplicate the full grammar in plugin or
website prose.

## Implementation notes
- Execution capability: highest available local capability; this changes the public agent-facing CLI and composes security-sensitive adapters.
- Review weight: standard (source: autopilot project default).
- Files changed: `crates/cli/src/command.rs`, `crates/cli/src/dispatch.rs`, `crates/cli/src/entrypoint.rs`, command/compiled-binary tests, `crates/core/src/runtime/artifact.rs`.
- Tests added: bootstrap grammar/help, isolated fixture install, same-major no-op, blocked major upgrade, unknown-version safety, harness attention separation, and failure-preserving artifact paths.
- Discrepancies from design: release resolution accepts a strict local manifest override for isolated environments and otherwise uses the bounded canonical GitHub latest endpoint; existing executable versions are probed directly and unknown versions block replacement.
- Adjacent issues parked: none.

## Review resolution
- Reconnected the binary path to `FileReleaseResolver`/`SystemReleaseResolver`, `ArtifactFetcher`, and `BinaryInstaller`.
- Added GitHub latest-release and checksum parsing, direct version probing, major-safe decisioning, and isolated fixture coverage for install, no-op, and blocked major paths.

## Review findings (2026-07-12)

- **Blocker**: The public command does not compose or execute the binary
  bootstrap boundary. `execute_system_bootstrap` only checks whether
  `SKILLTAP_RELEASE_MANIFEST` is present, reports `planned` when it is set,
  and never resolves a release manifest, selects the platform artifact,
  fetches it, verifies its checksum, installs it atomically, or probes the
  installed identity. The environment variable is not even read as a
  manifest path or transport input. Consequently `--allow-major` has no
  effect, fresh installs and same-major updates cannot succeed, major updates
  cannot be blocked based on an observed version, and the command can report
  an apparently planned binary operation without any corresponding mutation
  or verified result. Wire the existing `ReleaseResolver`, artifact fetcher,
  and `BinaryInstaller` ports (or an equivalent application composition) into
  the command, add isolated first-install/update/no-op/blocked-major and
  failure-preservation coverage, and report the actual binary result before
  advancing this story.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: binary bootstrap boundary is disconnected from the command (this item)
**Important**: none
**Nits**: none

**Notes**: Substrate review at standard weight, escalated to a focused
correctness/public-contract pass because this is the agent-facing bootstrap
command and a security-sensitive release boundary. Reviewed commit `176e812`,
the bootstrap feature design, `docs/SPEC.md`, `docs/ARCH.md`, the core artifact
ports, harness adapter, CLI implementation, help output, and the compiled
attention test. The CLI grammar and separate per-harness result shape are
present, but the only binary-path test proves the deliberate unavailable
fallback; setting `SKILLTAP_RELEASE_MANIFEST` merely changes the label to
`planned` and still performs no binary operation. The item remains at
`stage: implementing` until the command is wired to verified artifact
resolution/installation and its acceptance matrix is covered.

## Review findings (2026-07-12)

- **Blocker — success is reported before binary identity/version verification** (`crates/cli/src/entrypoint.rs:551-569`): after checksum publication the command immediately emits `installed`/`updated`; it never probes the newly published executable or checks that its reported version matches the selected release. The compiled tests use the literal `test-binary`, so they cannot catch this contract violation. A valid checksum for the wrong or non-executable file is currently reported as a successful bootstrap. Compose a bounded post-publish probe (and preserve/rollback the prior destination on failure) and add first-install/update regression coverage.
- **Blocker — production command accepts unrestricted local release/artifact overrides** (`crates/cli/src/entrypoint.rs:445-455, 535-543`): `SKILLTAP_RELEASE_MANIFEST` switches the command from the canonical latest-release resolver to any absolute local file, and `SKILLTAP_RELEASE_ARTIFACT` copies any local path into the install flow. These fixture seams are live in the shipped binary, bypass the HTTPS/redirect/checksum transport contract, and let an ambient environment choose arbitrary executable bytes. Keep deterministic fixture injection behind a test-only composition boundary (or a tightly authenticated local resolver) so normal `bootstrap` always resolves the canonical release and never trusts ambient source/artifact paths.
- **Important — installed-version override can produce false policy decisions** (`crates/cli/src/entrypoint.rs:469-474`): `SKILLTAP_INSTALLED_VERSION` wins over probing the existing executable. An ambient value can make an old/unknown binary appear current or newer, causing a false no-op and bypassing the intended verified major-version decision. Remove this production override and derive the version from the executable (or trusted skilltap state) only.
- **Important — workspace lint verification is failing** (`crates/core/src/bootstrap.rs:304-314`, `crates/core/src/runtime/artifact.rs:514-528`): `cargo clippy --workspace --all-targets --offline -- -D warnings` fails on the bootstrap changes, so the command story's recorded verification is incomplete even though the full test suite passes.

## Review (2026-07-12, follow-up)

**Verdict**: Request changes

**Blockers**: post-publish binary verification; removal of unrestricted production release/artifact overrides (this item)
**Important**: ambient installed-version override; clippy gate failures (this item)
**Nits**: none

**Notes**: Substrate review at standard weight, escalated to a public CLI/security pass after the prior binary-wiring fix. `cargo test --workspace --all-targets --offline` passed, but the command still reports success without the required verified executable identity and exposes fixture environment seams in the production composition. Item remains at `stage: implementing` pending fixes and isolated tests that exercise the real release boundary.

## Review (2026-07-12, hardened follow-up)

**Verdict**: Request changes

**Blockers**: composed bootstrap still depends on an identity-unsafe artifact
rollback boundary (tracked by the artifact correction item)
**Important**: the required compiled bootstrap acceptance matrix is absent
(this item)
**Nits**: none

**Notes**: Standard fresh-context substrate review of commits `c880496` and
`85b56ea`. The public grammar, target narrowing, separate binary/harness
result entries, canonical source, direct version probe, major guard, and
production removal of local release/artifact/version override seams are
present; workspace tests, clippy, and formatting are green. The hardened
follow-up removed all compiled bootstrap fixture tests, leaving no isolated
coverage for first install, same-major repeat, blocked major, target
narrowing, absent harness, mixed success/attention, failed pre-publish
preservation, or post-publish identity failure. Restore a test-only
composition seam (or equivalent deterministic fixture harness) and cover the
acceptance matrix without reintroducing ambient production overrides. The
item remains at `stage: implementing`.
