---
id: gate-security-workflow-action-pinning
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

# Pin release and deployment actions to immutable revisions

## Severity

High

## Domain

CI and supply-chain security

## Location

- `.github/workflows/release.yml:14-17,26,56,79,83,93,102,118,124,143`
- `.github/workflows/ci.yml:14,37,54,56`
- `.github/workflows/deploy.yml:25,29,39,45,58`

## Evidence

Release, CI, and deployment workflows reference mutable third-party action
tags while release jobs hold write, identity, attestation, and Homebrew-token
credentials.

## Remediation direction

Pin every third-party action to a reviewed full commit SHA, update pins through
reviewed dependency automation, minimize job permissions, and isolate the
Homebrew token to the job that requires it.
