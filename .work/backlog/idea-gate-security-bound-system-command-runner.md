---
id: idea-gate-security-bound-system-command-runner
created: 2026-07-15
updated: 2026-07-15
tags: [security, cleanup]
release_binding: null
gate_origin: security
---

# Bound the remaining Git command runner

## Severity
Low

## Relevance
Release-relevant to `3.1.0`; deferred by the configured low-severity backlog route.

## Domain
Infrastructure & Deployment

## Location
`crates/core/src/runtime/command.rs:116`

## Evidence

`SystemCommandRunner`, used for Git-root discovery, launches a direct argument vector but does not clear the inherited environment, set a deadline, cap stdout/stderr, or isolate the process group. Native harness processes already receive those controls through `SystemNativeProcessRunner`.

## Remediation direction

Route Git-root discovery through the bounded process runner or give `SystemCommandRunner` equivalent environment, deadline, output-limit, and termination controls. Preserve direct arguments and add regression coverage for timeout and output exhaustion.
