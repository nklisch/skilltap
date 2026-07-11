---
id: epic-rust-control-plane-workspace-reset
kind: feature
stage: review
tags: [infra, cleanup]
parent: epic-rust-control-plane
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Rust Workspace Reset

## Brief

Replace the retired TypeScript, Bun, and npm product implementation with a
buildable Rust Edition 2024 workspace containing the four architectural crates.
Pin the stable toolchain, establish dependency direction and baseline quality
checks, and provide the test-support skeleton needed by subsequent features.

The cleanup also rewrites build, CI, release, installer, and Homebrew plumbing
where it assumes the old runtime. The website, Homebrew distribution, installer,
and release experience remain product surfaces; this feature does not redesign
their content or implement v3 resource-management behavior.

## Epic context

- Parent epic: `epic-rust-control-plane`
- Position in epic: foundation feature — every other control-plane feature
  depends on the clean workspace and crate graph it establishes
- Inherits the clean-reset and pinned-stable decisions recorded by the parent

## Foundation references

- `docs/ARCH.md` — Workspace, Dependency Direction, Testing, Technology
- `docs/VISION.md` — Non-Goals, Principles
- `AGENTS.md` — Architecture, Development

## Design decisions

- **What version begins the clean-break Rust line?** Set the workspace package
  version to `3.0.0`; Cargo workspace metadata is the implementation source of
  truth for the binary version.
- **Does removing Bun/npm remove the website toolchain?** Remove JavaScript
  tooling from the product root and all production packages. Retain VitePress
  as an isolated website-only npm workspace with its own lockfile and Node
  scripts; it cannot be imported by or participate in the Rust product build.
- **How much CLI exists in the reset?** Provide only deterministic `--help` and
  `--version` behavior so release and installer surfaces have a real binary.
  The later CLI-shell feature owns the v3 command tree and output envelopes.
- **How are release targets built?** Use native GitHub-hosted runners for Linux
  x64/arm64 and macOS x64/arm64, preserving the existing asset names. Do not
  add an opaque cross-compilation wrapper.

## Architectural choice

Use a destructive root reset with one explicit exception for the isolated
website toolchain. The rejected alternatives were retaining the TypeScript
monorepo beside Rust, which would preserve stale authority and duplicate build
contracts, and deleting all distribution surfaces, which contradicts the
approved website/Homebrew/installer continuity decision.

The Rust root is a Cargo resolver-3 workspace with four crates matching
`docs/ARCH.md`. Shared package metadata lives in `[workspace.package]` and
shared dependencies live in `[workspace.dependencies]`. Production dependency
direction is expressed by Cargo manifests: CLI depends on core and harnesses,
harnesses depends on core, and production crates never depend on test-support.

## Implementation units

### Unit 1: Destructive cleanup and pinned workspace

**Story:** `epic-rust-control-plane-workspace-reset-workspace`

**Files:** `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `crates/*/Cargo.toml`,
`crates/*/src/*.rs`, plus deletion of the retired product/tooling paths.

```toml
[workspace]
members = ["crates/core", "crates/harnesses", "crates/cli", "crates/test-support"]
resolver = "3"

[workspace.package]
version = "3.0.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/nklisch/skilltap"
```

```rust
// crates/core/src/lib.rs
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// crates/cli/src/main.rs
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "skilltap", version, about = "Manage local agent environments")]
struct Cli {}

fn main() {
    Cli::parse();
}
```

**Implementation notes:**

- Pin toolchain `1.96.0` with `rustfmt` and `clippy`; omit `rust-version` until
  an MSRV is intentionally declared.
- Delete `packages/`, root Bun/npm/Biome/TypeScript manifests, old patches,
  product build scripts, committed legacy `.agents` state, stale Claude-only
  rules, and generated product artifacts. Preserve `.agents` skills/rules,
  `.research`, `.work`, foundation docs, website, installer, Homebrew, and
  GitHub workflow directories for their dedicated rewrite stories.
- Generate and commit `Cargo.lock`; build with warnings denied in CI rather
  than weakening crate-level lint settings.

**Acceptance criteria:**

- [ ] `cargo metadata --no-deps` reports exactly the four architectural crates.
- [ ] `cargo test --workspace` and `cargo build --release -p skilltap` pass.
- [ ] `target/release/skilltap --version` reports `skilltap 3.0.0`.
- [ ] No production TypeScript package or root Bun/npm toolchain remains.
- [ ] Cargo manifests enforce the documented production dependency direction.

### Unit 2: Rust quality and binary verification

**Story:** `epic-rust-control-plane-workspace-reset-ci`

**Files:** `.github/workflows/ci.yml`, `scripts/verify-binary.sh`.

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p skilltap
scripts/verify-binary.sh target/release/skilltap
```

**Implementation notes:** Keep verification synchronous and exercise the
compiled artifact. The smoke script accepts an optional binary path and checks
only `--version` and `--help` until later command features extend it.

**Acceptance criteria:**

- [ ] CI contains no Bun/npm product steps.
- [ ] Formatting, clippy, tests, release build, and compiled-binary smoke run on
  pushes and pull requests.
- [ ] The smoke script fails for a missing binary or wrong version/help exit.

### Unit 3: Release, installer, and Homebrew continuity

**Story:** `epic-rust-control-plane-workspace-reset-distribution`

**Files:** `.github/workflows/release.yml`, `install.sh`, `scripts/install-local.sh`,
`homebrew-skilltap/Formula/skilltap.rb`, and its formula-update automation.

```text
skilltap-linux-x64
skilltap-linux-arm64
skilltap-darwin-x64
skilltap-darwin-arm64
checksums.txt
```

**Implementation notes:** Build each binary on a native runner, strip where
appropriate, smoke-test before upload, attest release artifacts, remove npm
publication, and retain tag-driven GitHub releases. The curl installer and
Homebrew formula consume the unchanged asset naming contract.

**Acceptance criteria:**

- [ ] Release workflow builds and smoke-tests all four supported artifacts.
- [ ] Release workflow contains no npm publication or Bun runtime dependency.
- [ ] Installer maps supported OS/architecture pairs to the exact asset names
  and preserves `SKILLTAP_INSTALL` behavior.
- [ ] Homebrew formula description reflects the v3 control plane and its test
  invokes `skilltap --version`.

### Unit 4: Isolated current-state website

**Story:** `epic-rust-control-plane-workspace-reset-website`

**Files:** `website/package.json`, `website/package-lock.json`,
`website/scripts/gen-llms-txt.mjs`, and current website Markdown/configuration.

```json
{
  "scripts": {
    "dev": "vitepress dev",
    "build": "node scripts/gen-llms-txt.mjs && cp ../install.sh public/install.sh && vitepress build",
    "preview": "vitepress preview"
  }
}
```

**Implementation notes:** Replace the Bun-specific generator with portable
Node ESM, remove pages and navigation that describe the retired installer,
discovery, scanner, TUI, migration, or TypeScript architecture, and retain a
concise website aligned to the five foundation docs. Do not duplicate detailed
native contracts when a foundation link suffices.

**Acceptance criteria:**

- [ ] `npm --prefix website ci` and `npm --prefix website run build` pass.
- [ ] Website source and generated LLM text describe only the v3 product.
- [ ] Root Cargo commands do not install or execute website dependencies.
- [ ] Generated `public/install.sh` matches root `install.sh`.

## Implementation order

1. `epic-rust-control-plane-workspace-reset-workspace`
2. In parallel after the workspace: CI, distribution, and website stories

## Testing

- Workspace story: Cargo metadata, build, unit tests, version/help smoke, and
  repository residue searches.
- CI story: execute every workflow command locally and negative-test the smoke
  script with a nonexistent path.
- Distribution story: syntax-check shell scripts, validate workflow YAML, and
  assert the release/formula/installer asset-name matrix is identical.
- Website story: clean npm install, VitePress build, deterministic LLM output,
  and stale-surface searches.

## Risks

- GitHub-hosted runner labels and availability can change; the workflow keeps
  the four artifact names stable even if a runner label later needs updating.
- Keeping VitePress creates an intentional website-only Node dependency. Root
  manifests and CI make that boundary visible so it cannot become product
  runtime authority again.
- The destructive story deletes most of the repository. Its acceptance search
  and explicit preservation list are the guard against removing research,
  substrate, or distribution assets needed by later stories.

## Implementation summary (2026-07-11)

Children complete. The repository now has the pinned four-crate Rust workspace,
Rust-native CI and release workflows, aligned installer/Homebrew distribution,
and an isolated Node/npm VitePress site containing only the v3 control-plane
model. All four child stories are done and the integrated quality, binary,
shell, workflow, Ruby, website-build, installer-equality, and stale-surface
checks pass.
