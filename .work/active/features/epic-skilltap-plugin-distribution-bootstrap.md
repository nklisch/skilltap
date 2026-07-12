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

Provide the platform-aware bootstrap assets that make a published skilltap
plugin useful on supported macOS and Linux systems. The bootstrap must select
the correct release artifact, verify its checksum and expected version, place
it in a user-owned location, and report binary availability separately from
native plugin installation. Repeating a healthy setup must be a no-op, while
failed downloads or verification leave no misleading partial installation.

The feature must account for the native contract gap: Codex does not have an
attested non-interactive plugin mutation or post-install hook, and Claude's
trust/consent remains native. It therefore supplies a deterministic
agent-invocable path and uses native hooks only when capability evidence makes
that safe. It never edits harness caches, requires root, or turns arbitrary
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

<!-- Feature design will settle the bootstrap invocation boundary and artifact
transport without assuming undocumented harness behavior. -->
