---
id: epic-skilltap-plugin-distribution-release
kind: feature
stage: done
tags: [infra, content]
parent: epic-skilltap-plugin-distribution
depends_on: [epic-skilltap-plugin-distribution-package, epic-skilltap-plugin-distribution-cli-contract, epic-skilltap-plugin-distribution-bootstrap, epic-skilltap-plugin-distribution-guidance]
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Versioned Plugin and Binary Release

## Brief

Integrate the canonical plugin and verified bootstrap into skilltap's release
story. One release must publish the channel metadata, skill artifact,
platform binaries, checksums, and provenance with a consistent version and
source identity. CI should validate native package shape, help/bootstrap
contracts, and release inputs before publication, while the website, install
script, and Homebrew instructions describe the same path.

The website's plugin marketplace instructions and the one-line online
installer are equal first-class installation methods. The installer detects
installed Claude Code and Codex binaries and invokes the same bootstrap flow
to install or repair the skilltap resources they can load. Release metadata
also drives the opt-out/latest-compatible binary update lifecycle, with major
updates requiring explicit opt-in.

The release must validate both marketplace publishers: this repository's
native catalogs and the active `../skills` catalog entry. The sibling entry
points directly at this repository's plugin subdirectory, so both publishers
resolve the same identity and release without copied manifests or version
lockstep. The legacy `nklisch/skilltap-skills` repository is not a publisher to
preserve after cutover.

This feature owns release automation and public documentation alignment; it
does not redefine the native plugin schemas, write harness caches, or archive
the legacy `nklisch/skilltap-skills` repository before the canonical publication
has been verified.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: final publication integration; depends on all package,
  CLI, bootstrap, and guidance contracts.

## Foundation references

- `docs/SPEC.md` — Self-Hosted Plugin Distribution, Platform Contract,
  Validation
- `docs/ARCH.md` — Plugin Publication Boundary, Testing
- `.github/workflows/ci.yml` and `.github/workflows/release.yml`
- `install.sh`, `website/`, and `homebrew-skilltap/`
- `README.md` — installation and operating story

## Design decisions

- **Equal installation story**: Website marketplace instructions and the
  one-line installer are first-class peers. Both install/repair the binary and
  detected-harness skill/plugin resources through the same bootstrap boundary.
- **Update publication**: Release metadata drives latest-compatible default
  updates, user opt-out, and explicit major-version opt-in; the plugin skill,
  website, installer, and daemon must describe the same policy.
- **Secondary publisher pointer**: The active `../skills` marketplace remains
  a supported distribution path. Release checks fail when its skilltap entry
  does not point at this repository's canonical plugin subdirectory rather
  than attempting to synchronize copied metadata.

<!-- Feature design will define the exact release checks and publication
artifacts. -->

## Architectural choice

Treat the release as one versioned publication graph with generated/checkable
edges rather than hand-maintained copies. The canonical plugin tree and Cargo
workspace version feed channel manifests/catalogs, platform artifacts,
checksums, attestations, installer metadata, website instructions, and
Homebrew update inputs. CI validates the graph before publication; the release
workflow publishes only after package, bootstrap, and help contracts pass.
The active sibling `../skills` publisher is validated as a direct source
pointer to `nklisch/skilltap`'s `plugin/` subtree, never synchronized by copying
metadata or versions. The legacy repository remains cutover work, not a release
side effect.

Alternatives considered:

1. Keep release metadata and website copy independently maintained. This is
   familiar but permits version/source drift and mismatched install behavior.
2. Generate every website and Homebrew file from CI. This reduces drift but
   makes public documentation opaque and expands release tooling unnecessarily.
3. Chosen: keep human-readable docs and formulas in their owning repos while
   adding deterministic parity checks and one canonical installer/bootstrap
   boundary.

The riskiest unit is installer/bootstrap parity: the shell installer must not
become a second binary publisher or bypass artifact identity checks while still
detecting Claude/Codex and reporting partial harness setup. It delegates to the
verified Rust bootstrap boundary and validates the structured result.

## Implementation Units

### Unit 1: Release identity and artifact contract

**Files**: `.github/workflows/release.yml`, `scripts/verify-binary.sh`,
`plugin/.claude-plugin/*`, `plugin/.codex-plugin/*`, package fixtures
**Story**: `story-skilltap-plugin-distribution-release-contract`

Add deterministic release checks for workspace/plugin/catalog version parity,
canonical source identity, platform asset names, checksums, executable version,
and attestations. Keep the native channel schemas distinct while requiring one
public `skilltap` identity. Validate package/help/bootstrap contracts before
upload and make release artifacts consumable by the Rust latest-release
resolver.

**Acceptance criteria**:

- [ ] A tag is accepted only when it matches Cargo and both channel metadata
      versions.
- [ ] Every supported platform has exactly one verified executable asset and a
      checksums entry; provenance attestations cover uploaded assets.
- [ ] Release checks reject source identity drift or a malformed package before
      publication and never write harness caches.

### Unit 2: Installer and bootstrap parity

**Files**: `install.sh`, installer tests, `.github/workflows/ci.yml`
**Story**: `story-skilltap-plugin-distribution-release-installer`

Make the one-line installer a peer of marketplace installation. It downloads
and verifies the latest binary, delegates install/repair to the same bootstrap
flow, detects installed Claude/Codex binaries without enabling them, and keeps
binary and per-harness outcomes separate. It must preserve an existing binary
on failed verification and tolerate harness attention while failing on binary
attention.

**Acceptance criteria**:

- [ ] Installer and `skilltap bootstrap` use the same release/checksum/version
      rules and bounded redirect policy.
- [ ] Detection of installed harness executables is isolated and never implies
      `harness enable` or writes native caches.
- [ ] Re-running a healthy installer is idempotent; failed download, checksum,
      version, permission, or bootstrap publication preserves the prior binary.
- [ ] Offline shell tests cover supported platforms, redirects, malformed
      metadata, and mixed harness attention.

### Unit 3: Website, Homebrew, and secondary publisher parity

**Files**: `website/guide/getting-started.md`, `website/guide/updates.md`,
`website/reference/*`, `README.md`, `homebrew-skilltap/*`, active `../skills`
marketplace entry check
**Story**: `story-skilltap-plugin-distribution-release-install-surfaces`

Rewrite public installation guidance so marketplace plugin installation and the
online one-line installer are equal paths. Explain bootstrap, binary/harness
result separation, update opt-out/major policy, and Homebrew's binary-only
relationship. Add a check that the active sibling marketplace points directly
to this repository's `plugin/` subtree; do not archive or modify the active
repository as part of this feature.

**Acceptance criteria**:

- [ ] Website and README describe native plugin installation beside the online
      installer with matching bootstrap and update semantics.
- [ ] Homebrew instructions and formula remain a supported binary path and do
      not claim to install harness plugins automatically.
- [ ] A parity test fails when `../skills`'s skilltap entry is not the direct
      canonical source pointer.
- [ ] Website build and link checks pass without generated dist files becoming
      a second source of truth.

### Unit 4: Release workflow and publication verification

**Files**: `.github/workflows/ci.yml`, `.github/workflows/release.yml`,
`scripts/`, release docs
**Story**: `story-skilltap-plugin-distribution-release-verification`

Wire package validation, compiled help/JSON tests, bootstrap fixture tests,
website build, checksum generation, and artifact attestation into the release
gates. Verify the release manifest shape consumed by bootstrap and emit clear
failure boundaries. Keep publication/tag/push authority outside this feature's
implementation run; CI prepares and validates publication.

**Acceptance criteria**:

- [ ] CI runs Rust format/lint/tests, package/guidance validation, website
      build, and platform binary verification before release assets publish.
- [ ] Release assets, checksums, source identity, and attestations are
      reproducible and consumed by latest-compatible bootstrap resolution.
- [ ] A dry-run/fixture verification path is offline and cannot touch caller
      state, credentials, or native caches.

## Implementation Order

1. `story-skilltap-plugin-distribution-release-contract`
2. `story-skilltap-plugin-distribution-release-installer` and
   `story-skilltap-plugin-distribution-release-install-surfaces` (parallel
   after the contract and guidance references are stable)
3. `story-skilltap-plugin-distribution-release-verification` (after the above
   paths exist; owns final CI/release parity wiring)

## Testing

- Offline Rust tests validate package/skill/help/bootstrap contracts and all
  release-manifest branches with temporary roots.
- Shell installer tests use fake curl/release/harness binaries and assert direct
  vectors, checksums, redirect hops, cleanup, prior-binary preservation, and
  mixed binary/harness results.
- Website build/link checks validate install and marketplace pages against the
  current command/help contract.
- A sibling-pointer fixture checks the active `../skills` marketplace entry
  without mutating that repository.

## Risks

- Shell and Rust bootstrap behavior can drift if either gains independent
  release parsing. Keep Rust as the verification authority and test the shell's
  delegation/result interpretation.
- GitHub release asset naming and checksum layout are consumed by bootstrap;
  treat a naming change as a contract change requiring synchronized tests.
- Website generated output can hide stale source docs. Build from source and
  review only checked-in source pages, not generated `dist` as a new authority.
- Homebrew is maintained in a separate repository. The workflow may prepare a
  pull request but must not assume it can atomically update external state in
  this workspace.

## Design decisions

- **Publisher model**: this repo is canonical; `../skills` points directly at
  `./plugin` from this repo and remains active; no duplicate lockstep metadata.
- **Installer behavior**: verify first, delegate to Rust bootstrap, report
  binary/harness results separately, and fail without replacing a healthy
  existing binary.
- **Major updates**: auto-safe within the current major only; opt-out remains
  supported and major updates require explicit `--allow-major`.
- **Cutover boundary**: legacy `nklisch/skilltap-skills` archival belongs to
  the dependent cutover feature after publication verification.

## Children complete

The four release stories are complete: release identity/artifact contract,
installer/bootstrap parity, website/Homebrew/secondary-publisher installation
surfaces, and pre-publication workflow verification.

## Review (2026-07-12)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: none
**Nits**: the active sibling publisher remains an external read-only parity
input; CI enforces it when `SKILLTAP_SKILLS_MARKETPLACE` points at a checkout.

**Notes**: Feature acceptance review at standard weight. Release identity,
platform/checksum/provenance gates, installer/bootstrap parity, public
marketplace/online/Homebrew guidance, and offline validation fixtures are
aligned. The legacy sibling archive remains explicitly deferred to cutover.

## Review follow-up (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fresh aggregate review confirmed installer redirect/checksum and
binary/harness result handling, release workflow ordering, website/Homebrew
parity, and the explicit read-only sibling-check boundary. Hardened
`verify-release-contract.sh` to reject non-canonical manifest repositories or
native catalog source roots directly, in addition to the package validator.
Offline release-contract, installer, installation-surface, and package checks
pass. The optional sibling checkout remains an explicit external parity input;
legacy retirement remains deferred to cutover.
