---
id: story-skilltap-plugin-distribution-release-contract
kind: story
stage: review
tags: [infra, security, testing]
parent: epic-skilltap-plugin-distribution-release
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Enforce release identity and artifact contracts

Add deterministic checks that tie Cargo, native channel manifests/catalogs,
platform asset names, checksums, executable identity, and provenance to one
release. Validate the canonical plugin tree and bootstrap manifest shape before
publication without mutating native harness state.

Acceptance criteria:

- Release tags match the workspace and both native channel metadata versions.
- Supported platform assets are unique, executable, checksummed, and named for
  the latest-release resolver; attestations cover uploaded artifacts.
- Source identity drift and malformed package inputs fail before publication.

## Implementation notes
- Execution capability: highest; release identity and supply-chain boundary.
- Review weight: standard (autopilot caller policy).
- Files changed: `scripts/verify-release-contract.sh`, `.github/workflows/release.yml`.
- Tests added: offline release contract script checks channel identity/version, supported asset matrix, checksums, and provenance attestation.
- Discrepancies from design: none.
- Adjacent issues parked: none.
