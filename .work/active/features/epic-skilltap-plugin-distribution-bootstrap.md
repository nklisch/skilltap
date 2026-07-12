---
id: epic-skilltap-plugin-distribution-bootstrap
kind: feature
stage: drafting
tags: [infra, security]
parent: epic-skilltap-plugin-distribution
depends_on: [epic-skilltap-plugin-distribution-package]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Verified Skilltap Binary Bootstrap

## Brief

Provide the platform-aware bootstrap flow that makes a published skilltap
plugin useful on supported macOS and Linux systems. Bootstrap ensures the
skilltap binary is installed or repaired, selects and verifies the current
release artifact, detects installed Claude Code and Codex executables, and
installs or repairs the skilltap skill/plugin resources those harnesses can
load. It reports binary availability and harness resource setup separately.
Repeating a healthy setup must be a no-op, while failed downloads or
verification leave no misleading partial installation.

The feature must account for the native contract gap: Codex does not have an
attested non-interactive plugin mutation or post-install hook, and Claude's
trust/consent remains native. It therefore supplies a deterministic
agent-invocable path and uses native hooks only when capability evidence makes
that safe. The same boundary is callable by the one-line online installer
after binary verification. Binary updates default to the latest compatible
release, can be opted out of, and never auto-apply a major version unless the
user opts in. It never edits harness caches, requires root, or turns arbitrary
plugin installation into code execution.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: consumer of the package contract; release integration and
  final guidance depend on its verified bootstrap behavior.

## Foundation references

- `docs/SPEC.md` — Self-Hosted Plugin Distribution, Mutation Safety, Platform
  Contract, Validation
- `docs/ARCH.md` — Plugin Publication Boundary, Native Command Execution,
  Testing
- `install.sh` — existing platform and checksum protocol
- `.github/workflows/release.yml` — release assets, checksums, and attestations
- `crates/test-support/` — isolated machine and native-process fixtures

## Design decisions

- **Bootstrap responsibility**: The first-class bootstrap flow installs or
  repairs the latest binary, detects installed Claude Code and Codex binaries,
  and installs or repairs the skilltap resources those harnesses can load. It
  does not adopt environments, enable ordinary harness management, or mutate
  unrelated resources.
- **Online installer parity**: `install.sh` invokes the same bootstrap flow
  after binary verification, so marketplace installation and the one-line
  website install are equivalent setup methods.
- **Update policy**: Bootstrap/manual update fetches the latest release. The
  ongoing auto-update policy is enabled by default but opt-out; it applies
  within the current major version and requires explicit opt-in for major
  upgrades.

<!-- Feature design will settle the bootstrap command surface and artifact
transport without assuming undocumented harness behavior. -->
