---
id: gate-security-credential-bearing-source-state
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

# Prevent credential-bearing source locators from entering state

## Severity

High

## Domain

Secrets and data protection

## Location

- `crates/core/src/domain/source.rs:10,31-49`
- `crates/core/src/domain/resource.rs:331-364`
- `crates/core/src/storage/state.rs:274-317`
- `crates/cli/src/application.rs:1457-1465,2189-2202`
- `crates/core/src/storage/repository.rs:307-324`

## Evidence

`SourceLocator` accepts locator text such as `https://user:token@host/repo.git`,
and desired/state serialization persists it to `inventory.toml` and `state.json`
without credential redaction or restrictive file-mode enforcement.

## Remediation direction

Reject or normalize URL userinfo and credential-bearing locators, use credential
helpers or environment references, store only redacted locators, enforce 0600
document files and 0700 configuration/managed directories, and add persistence
secret-canary tests.

## Implementation notes

- Execution capability: highest same-harness capability; this is a security-sensitive domain and persistence boundary.
- Review weight: standard (caller/default).
- Files changed: `crates/core/src/domain/{mod.rs,source.rs}`, `crates/core/src/runtime/filesystem.rs`, `crates/core/src/runtime/filesystem/directory_tree/unix_support.rs`, `crates/core/src/storage/repository.rs`, `crates/core/tests/storage_integration.rs`, `crates/cli/src/command/tests.rs`.
- Tests added: URI credential and credential-query rejection with persisted-wire coverage; CLI source-argument rejection; Unix mode assertions for owned documents and managed artifact trees.
- Discrepancies from design: credential-bearing locators fail closed rather than being normalized/redacted, preserving the no-authentication-material invariant while directing callers to Git helpers or environment-backed authentication. SCP-style Git locators remain valid because their `user@host:path` form is not URI userinfo.
- Adjacent issues parked: none.
- System persistence now creates/replaces owned documents with mode `0600`, configuration roots with mode `0700`, and managed artifact roots are hardened to `0700` when opened.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard substrate review with deep security and persistence-boundary
lenses. URI userinfo and common credential query keys are rejected before
serialization, SCP-style Git locators remain supported, and owned documents,
configuration roots, and managed trees are hardened to user-only modes. Focused
source, CLI validation, storage integration, and Unix permission tests all pass.
