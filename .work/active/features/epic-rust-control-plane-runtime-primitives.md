---
id: epic-rust-control-plane-runtime-primitives
kind: feature
stage: drafting
tags: [infra]
parent: epic-rust-control-plane
depends_on: [epic-rust-control-plane-domain-contracts]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Runtime Boundary Primitives

## Brief

Provide reusable ports and platform adapters for global/project scope
resolution, target resolution, canonical paths, atomic filesystem operations,
process-wide fail-fast configuration locking, time, and direct executable-plus-
argument-vector command invocation. Return typed boundary errors and captured
command evidence without writing to the terminal or leaking secrets.

These primitives support later harness adapters and reconciliation execution,
but do not encode Codex or Claude commands, semantic planning, or resource
lifecycle behavior. Synchronous operation remains the default unless measured
behavior later justifies concurrency.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: independent infrastructure consumer of the shared domain
  contracts; can proceed in parallel with storage

## Foundation references

- `docs/SPEC.md` — Operating Model, Mutation Safety, Platform Contract
- `docs/ARCH.md` — Native Command Execution, Concurrency, Error Model,
  Technology
- `AGENTS.md` — Architecture, Development

<!-- The feature design pass will fill in implementation units. -->
