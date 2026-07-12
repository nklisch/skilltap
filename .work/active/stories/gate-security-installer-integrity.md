---
id: gate-security-installer-integrity
kind: story
stage: done
tags: [security]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: security
created: 2026-07-12
updated: 2026-07-12
---

# Verify installer artifacts before installation

## Severity

High

## Domain

Supply chain and installation

## Location

- `install.sh:72-100`
- `install.sh:107-110`

## Evidence

The installer downloads the latest release asset from the GitHub API without
verifying `checksums.txt` or an attestation, then removes the macOS quarantine
attribute before execution.

## Remediation direction

Download and verify the expected SHA-256 or attestation before installation,
fail closed on mismatch, and stop clearing quarantine unless an explicit,
documented opt-in requires it.

## Implementation Notes

- `install.sh` now downloads the release `checksums.txt` alongside the
  platform asset and verifies the selected asset with `sha256sum` or the
  portable macOS `shasum -a 256` fallback before moving it into the install
  directory.
- Verification fails closed when the asset is absent from the checksum file,
  the digest is malformed, no SHA-256 utility is available, or the digest does
  not match. Temporary files are cleaned up by the exit trap.
- The installer no longer removes the macOS quarantine attribute.
- Verification: `sh -n install.sh`.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: release checksums are fetched from the same immutable release tag as
the artifact; a future hardening pass could add signed attestation verification,
but the stated SHA-256 fail-closed requirement is met.

**Notes**: Standard substrate review with deep supply-chain/security lenses.
The installer downloads the selected checksum manifest, requires an exact
64-hex digest for the selected asset, verifies with `sha256sum` or portable
`shasum`, aborts before installation on any mismatch/missing tool, and no
longer strips macOS quarantine. `sh -n install.sh` passes.
