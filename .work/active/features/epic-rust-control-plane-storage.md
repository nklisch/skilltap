---
id: epic-rust-control-plane-storage
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

# Machine State Storage

## Brief

Implement versioned repositories for `config.toml`, `inventory.toml`,
`state.json`, and skilltap-owned artifacts beneath the resolved machine-wide
configuration directory. Reads validate full documents and reject unknown
skilltap-owned fields; writes validate complete replacements and atomically
publish them so readers observe either the old or new document.

The repositories model missing first-use state, managed artifact ownership, and
recoverable backup locations without storing authentication material. This
feature does not observe harness-native files, calculate reconciliation plans,
or perform resource lifecycle operations.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: independent infrastructure consumer of the shared domain
  contracts; can proceed in parallel with runtime primitives

## Foundation references

- `docs/SPEC.md` — Configuration Directory, `config.toml`, `inventory.toml`,
  `state.json`, `managed/`, Validation
- `docs/ARCH.md` — Storage, Concurrency, Error Model
- `docs/VISION.md` — Core Idea, Audience, Observable Ownership

<!-- The feature design pass will fill in implementation units. -->
