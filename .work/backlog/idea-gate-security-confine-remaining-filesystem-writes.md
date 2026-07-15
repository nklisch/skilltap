---
id: idea-gate-security-confine-remaining-filesystem-writes
created: 2026-07-15
updated: 2026-07-15
tags: [security, cleanup]
release_binding: null
gate_origin: security
---

# Confine remaining top-level filesystem writes

## Severity
Low

## Relevance
Ambient defense-in-depth finding.

## Domain
Input Validation & Injection

## Location
`crates/core/src/runtime/filesystem.rs:207` and `crates/core/src/runtime/filesystem.rs:226`

## Evidence

`SystemFileSystem::atomic_write` and `create_relative_symlink` inspect a path before a separate rename or symlink operation. In-bundle callers hold the process-wide configuration lock, and higher-risk managed-tree operations already use descriptor-relative, no-follow `ConfinedFileSystem` operations, so this is not a release blocker.

## Remediation direction

Migrate remaining callers to descriptor-relative confined operations where practical, or revalidate destination identity with no-follow semantics immediately at publication. Preserve current lock discipline and rollback behavior.
