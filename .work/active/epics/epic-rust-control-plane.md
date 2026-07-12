---
id: epic-rust-control-plane
kind: epic
stage: done
tags: [infra, cleanup]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: null
created: 2026-07-10
updated: 2026-07-12
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
  and atomic writes through the runtime filesystem port — depends on:
  `[epic-rust-control-plane-runtime-primitives]`
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
v2 implementation assumptions. Runtime primitives own the canonical atomic
filesystem implementation; storage follows them so document repositories do
not grow a second writer. Both share domain errors and canonical-path types
from the preceding domain feature. The CLI shell is the integration
point and therefore the likeliest place to expose an omitted primitive; any
such discovery belongs in the producing feature contract rather than being
reimplemented in the CLI crate.

## Implementation summary

The clean-break Rust control plane is complete. The repository now contains a
pinned Edition 2024 Cargo workspace with legacy TypeScript/Bun product code
removed; strict validated domain contracts; platform/scope/target, direct
command, no-follow filesystem, locking, atomic document, and immutable managed
artifact runtime/storage boundaries; and a runnable non-interactive CLI with
the complete v3 grammar, stable plain/JSON outcomes, exact exits, read-only
first-use status, and explicit unavailable results for later native capability
epics. Complete skills remain whole directory trees, and no discovery,
scanning, migration, or native behavior is simulated.

The website, installer, Homebrew, CI, and four-platform release surfaces remain
and are aligned to the Rust build. Optimized binaries run smoke and full CLI
contracts before publication. Two maintainability cadences preserved public
identities and product behavior while decomposing runtime, storage, and CLI
pressure points. The locked workspace passes 192 tests, warnings-denied Clippy
and rustdoc, optimized build, and explicit compiled-binary verification.

## Completion review findings

Fresh-context completion review requested two corrections before approval:

1. The public website CLI reference and generated `llms-full.txt` still publish
   obsolete result labels, JSON fields, and exit codes instead of the
   authoritative schema-1 and `0`–`3` contract.
2. Pre-merge CI is Ubuntu-only, so Apple-specific descriptor, errno, locking,
   managed-artifact, and recovery paths run only after a release tag.

Additional children:

- `epic-rust-control-plane-website-cli-contract` — align the public website
  reference and regenerate ingestion output — depends on
  `[epic-rust-control-plane-cli-shell]`.
- `epic-rust-control-plane-macos-ci` — add a native macOS pre-merge locked
  workspace and optimized binary contract — depends on
  `[epic-rust-control-plane-cli-shell]`.

## Final review

Approved after both completion findings were corrected. The public website and
generated ingestion artifact now preserve foundation human labels while
documenting exact schema-1 JSON fields and exits `0`–`3`. Native `macos-14`
pre-merge CI runs the locked full workspace, optimized build, and compiled
binary wrapper without weakening Ubuntu, website, or release gates. The final
focused YAML, 192-test, optimized binary, website generation/build, and clean
tree checks pass; the epic is complete.
