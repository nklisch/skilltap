---
id: story-skilltap-plugin-distribution-bootstrap-artifacts
kind: story
stage: implementing
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-bootstrap
depends_on: [story-skilltap-plugin-distribution-bootstrap-contract]
release_binding: null
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
