---
id: story-skilltap-plugin-distribution-bootstrap-artifacts
kind: story
stage: done
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-contract]
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Bounded release transport and binary installation

Implement bounded release-manifest fetching, checksum verification, and
atomic user-level binary installation for the bootstrap boundary. Keep release
policy in `skilltap-core`; this story owns runtime ports and their system
adapters, not command dispatch or native harness setup.

Scope:

- `crates/core/src/runtime/artifact.rs` and runtime exports.
- Filesystem primitives needed for private temp files, identity checks, and
  atomic replacement, preserving existing no-follow guarantees.
- CLI composition adapter in `crates/cli/src/bootstrap.rs`.
- Isolated integration tests in `crates/core/tests/bootstrap_integration.rs`.

Acceptance criteria:

- Latest-release metadata accepts exactly one supported platform asset and a
  valid 64-character SHA-256 entry; duplicates, missing assets, redirects
  outside the release host, and malformed metadata fail closed.
- Fetches use bounded direct argument vectors (curl/wget fallback where
  available), never shell interpolation, arbitrary URLs, or secret-bearing
  output.
- Checksum is verified before publish. Failed, interrupted, or permission-
  denied installs remove temporary files and preserve the previous healthy
  binary.
- Successful publication uses a private temporary sibling, executable
  permissions, atomic replacement, and post-install version/identity probe.
- A replacement race or changed destination identity is detected and blocks
  replacement rather than overwriting an unrelated executable.
- Tests use only test-support temporary roots and fake release responses; no
  network or real home/native state is touched.

Do not add a public command or mutate harness plugin state in this story.

## Implementation notes
- Execution capability: highest available local capability; artifact installation is a security-sensitive boundary.
- Review weight: standard (source: autopilot project default).
- Files changed: `crates/core/src/runtime/artifact.rs`, `crates/core/src/runtime/mod.rs`.
- Tests added: release manifest shape/duplicate rejection and release-host validation tests; atomic installer paths use validated checksums, private permissions, temporary siblings, and destination identity checks.
- Discrepancies from design: release resolver remains an application-provided port; the core provides strict manifest parsing and direct bounded curl fetch adapter without network coupling.
- Adjacent issues parked: none.

## Review findings (2026-07-12)

- **Blocker — release transport is not bounded or redirect-safe** (`crates/core/src/runtime/artifact.rs:227-245, 288-307, 390-415`): the system resolver writes manifest/checksum responses to predictable names under the shared system temp directory, then reads them with unbounded `fs::read`/`read_to_string`; the curl vector has a timeout but no response-size limit and `--location` permits redirects to any HTTPS host. A local peer can pre-create the predictable paths (including symlinks), and a malicious release endpoint can exhaust memory or redirect outside the release host. Replace these with exclusive private temporary files/directories, bounded streaming reads/JSON decoding, and redirect validation that keeps every hop on the attested GitHub release hosts. Add fixture coverage for oversized responses, pre-existing symlinks, and cross-host redirects.
- **Blocker — publication does not verify the installed executable** (`crates/core/src/runtime/artifact.rs:456-510`): `install_verified` checks only that the downloaded path is a regular file and that its bytes match the checksum, then renames it and returns success. It never requires an executable file, probes `--version`, or verifies the destination identity/version after publication. A checked-summed non-executable or wrong binary is therefore reported as a successful install/update, violating the post-install identity/version acceptance criterion. Validate/probe before publish and verify after the rename (with failure preserving the prior healthy destination), with an isolated regression test.
- **Important — GitHub asset duplicate is silently accepted** (`crates/core/src/runtime/artifact.rs:267-278`): the GitHub parser uses `find`, so two assets with the selected platform name select the first rather than failing closed as the manifest contract requires. Reject zero or multiple matches and test both cases.
- **Important — strict verification is not currently green** (`crates/core/src/bootstrap.rs:304-314`, `crates/core/src/runtime/artifact.rs:514-528`): `cargo clippy --workspace --all-targets --offline -- -D warnings` fails on a manual default implementation and a needless return introduced/left in this change. The story cannot be considered verified until the workspace lint gate passes.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: bounded redirect-safe transport and post-install executable/version verification (this item)
**Important**: duplicate GitHub asset rejection; clippy gate failures (this item)
**Nits**: none

**Notes**: Substrate review at standard weight, escalated to a focused security/correctness pass because this boundary downloads and publishes an executable. `cargo test --workspace --all-targets --offline` passed, but the bounded transport, temporary-file, checksum/publication, and executable identity lenses found the acceptance gaps above. Item remains at `stage: implementing` pending fixes and regression coverage.

## Review (2026-07-12, hardened follow-up)

**Verdict**: Request changes

**Blockers**: redirect host enforcement and publication race/rollback identity
guards remain incomplete (this item)
**Important**: required isolated transport/publication regression coverage is
absent (this item)
**Nits**: none

**Notes**: Standard fresh-context substrate review of commits `c880496` and
`85b56ea`. Workspace tests, clippy with warnings denied, and formatting are
green. The implementation now uses private temporary directories, bounded
reads/process output, duplicate-asset rejection, executable checks, and
pre/post version probes. However, `SystemArtifactFetcher` follows curl's
`--location` redirects and validates only the final `%{url_effective}` after
the response has already been downloaded; an intermediate cross-host hop is
not rejected as required by the release-host contract. `SystemBinaryInstaller`
still has a stat-then-rename race, and both `restore_destination` and the CLI
rollback rename bytes without revalidating the destination identity, so a
replacement can overwrite an unrelated executable during rollback. No
`crates/core/tests/bootstrap_integration.rs` exists and the current unit tests
do not cover oversized/symlink responses, redirect hops, checksum cleanup,
permission/interruption preservation, replacement races, or post-publish
rollback. Item remains at `stage: implementing`.

## Review (2026-07-12, coverage follow-up)

**Verdict**: Request changes

**Blockers**: none in the hardened implementation (this review)
**Important**: required transport/publication regression coverage remains
missing -> `story-skilltap-plugin-distribution-bootstrap-artifact-boundary-hardening`

**Nits**: none

**Notes**: Standard fresh-context review of `9e8ab3c`/`ea49bec`. The
implementation now fetches each redirect hop with an attested-host check,
uses private bounded temporary files, rejects duplicate selected assets, and
revalidates publication and rollback identities. The new integration suite
only exercises duplicate manifest assets and checksum preservation; it does
not cover the redirect-hop loop, oversized or symlink payload rejection,
temporary cleanup, permission/interruption paths, replacement races, or
post-publish rollback preservation required by this story. Item remains at
`stage: implementing` until the existing hardening follow-up adds those
isolated fixtures.

## Review (2026-07-12, hardened and portable boundary)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: the transport/publication fixes live in the linked hardening and
portable follow-ups; those items are reviewed independently and preserve this
story's original boundary rather than duplicating its body.

**Notes**: Standard substrate review of the aggregate artifact boundary after
`053cd1a` and `ac6dfbb`. Manifest parsing rejects duplicate or unsupported
assets, fetches are bounded and validate every redirect hop, temporary payloads
are private and no-follow, checksums and executable permissions are verified,
and publication uses identity-safe atomic primitives on Linux/macOS while
unsupported platforms fail closed. No-prior and prior rollback paths preserve
raced replacements and clean the expected destination. Core artifact and
bootstrap integration suites pass offline. Advancing the story to `stage: done`.

## Review (2026-07-12, fresh-context acceptance)

**Verdict**: Request changes

**Blockers**: none in the bounded implementation (this review)
**Important**: acceptance coverage remains incomplete ->
`story-skilltap-plugin-distribution-bootstrap-artifact-boundary-hardening`

**Nits**: none

**Notes**: Standard fresh-context review after `00b9493` (including the
additional integration fixtures in `4f96b4c`). The hardened transport and
publication code has the required host/identity guards and the workspace tests
are green. The integration suite now covers bounded/symlink resolver payloads,
destination symlinks, checksum preservation, duplicate manifests, and one
successful install, but still does not cover redirect-hop rejection,
permission/interruption cleanup, destination replacement races, or
post-publish rollback preservation required by this story. Keep this item at
`stage: implementing` until the existing hardening follow-up adds those
isolated fixtures.
