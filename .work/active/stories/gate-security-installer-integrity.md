---
id: gate-security-installer-integrity
kind: story
stage: review
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
