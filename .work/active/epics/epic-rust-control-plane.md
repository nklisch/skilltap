---
id: epic-rust-control-plane
kind: epic
stage: implementing
tags: [infra, cleanup]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-11
---

# Rust Control Plane

## Brief

Deliver the clean-break Rust foundation on which every skilltap capability can
be built. This epic removes the retired TypeScript product surface, establishes
the Cargo workspace and crate boundaries, and provides the validated domain,
storage, scope, command, output, filesystem, and error primitives required by
later capabilities.

The result should be a small runnable CLI whose schemas and boundaries match the
v3 foundation. It does not implement native harness observation, reconciliation,
or resource lifecycle behavior; those capabilities build on this foundation.

## Foundation references

- `docs/VISION.md` — Core Idea, Agent Forward, Principles
- `docs/SPEC.md` — Operating Model, Configuration Directory, Output, Exit Codes
- `docs/ARCH.md` — Workspace, Dependency Direction, Domain Model, Storage, Technology

## Design decisions

- **How destructive is the repository reset?** Remove the legacy TypeScript,
  Bun, and npm product implementation and rebuild supporting infrastructure for
  Rust. Preserve the website, Homebrew, installer, and release experience as
  product surfaces, but rewrite their implementation and content where the old
  stack or v2 behavior leaks through.
- **What Rust compatibility policy applies initially?** Use Rust Edition 2024
  with a pinned stable toolchain. Defer a declared MSRV until v3 approaches
  release and its actual dependency floor is known.
- **Does this epic require UI mockups?** No. skilltap is a non-interactive CLI
  and daemon with no visual UI surface; do not install project mockup rules or
  create mockups for this epic.

## Decomposition

The epic is split by capability boundary rather than crate or implementation
layer. Repository reset establishes the buildable workspace; domain contracts
then give storage and runtime infrastructure one shared vocabulary. Storage and
runtime primitives can proceed in parallel before the CLI composes both into a
stable executable surface.

### Child features

- `epic-rust-control-plane-workspace-reset` — remove the retired product and
  establish the pinned Rust workspace, test-support skeleton, and rewritten
  build/distribution foundations — depends on: `[]`
- `epic-rust-control-plane-domain-contracts` — define validated identities,
  scopes, sources, resources, capabilities, compatibility, and result types —
  depends on: `[epic-rust-control-plane-workspace-reset]`
- `epic-rust-control-plane-storage` — implement versioned configuration,
  inventory, state, and managed-artifact repositories with strict validation
  and atomic writes — depends on: `[epic-rust-control-plane-domain-contracts]`
- `epic-rust-control-plane-runtime-primitives` — provide scope and target
  resolution, typed boundary errors, filesystem and locking abstractions, and
  direct-argument command execution — depends on:
  `[epic-rust-control-plane-domain-contracts]`
- `epic-rust-control-plane-cli-shell` — compose the repositories and runtime
  ports behind the non-interactive command tree, stable plain/JSON envelopes,
  and exit-code contract — depends on:
  `[epic-rust-control-plane-storage, epic-rust-control-plane-runtime-primitives]`

### Decomposition risks

The destructive reset is intentionally first but must preserve the website,
Homebrew, installer, and release experience as surfaces while removing their
v2 implementation assumptions. Storage and runtime primitives share domain
errors and canonical-path types; keeping those contracts in the preceding
domain feature avoids circular ownership. The CLI shell is the integration
point and therefore the likeliest place to expose an omitted primitive; any
such discovery belongs in the producing feature contract rather than being
reimplemented in the CLI crate.
