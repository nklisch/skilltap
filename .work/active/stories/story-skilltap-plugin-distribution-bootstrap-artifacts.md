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
- Review weight: standard (autopilot project default).
