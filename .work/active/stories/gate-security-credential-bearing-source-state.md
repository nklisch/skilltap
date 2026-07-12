---
id: gate-security-credential-bearing-source-state
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
