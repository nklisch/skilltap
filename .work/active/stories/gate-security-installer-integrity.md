---
id: gate-security-installer-integrity
kind: story
stage: implementing
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
