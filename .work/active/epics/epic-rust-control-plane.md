---
id: epic-rust-control-plane
kind: epic
stage: drafting
tags: [infra, cleanup]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
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

## Anticipated child features

- Destructive legacy removal and Rust workspace bootstrap
- Validated domain identities and resource graph
- Configuration, inventory, state, and managed-artifact repositories
- Scope, target, typed-error, locking, and command-runner primitives
- Thin CLI shell with stable plain and JSON output contracts

<!-- The design pass on each child feature will fill in real specifics. -->
