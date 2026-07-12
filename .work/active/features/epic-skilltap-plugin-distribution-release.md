---
id: epic-skilltap-plugin-distribution-release
kind: feature
stage: drafting
tags: [infra, content]
parent: epic-skilltap-plugin-distribution
depends_on: [epic-skilltap-plugin-distribution-package, epic-skilltap-plugin-distribution-cli-contract, epic-skilltap-plugin-distribution-bootstrap, epic-skilltap-plugin-distribution-guidance]
release_binding: null
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

The release must update and validate both marketplace publishers: this
repository's native catalogs and the active `../skills` catalog entry. They
publish the same plugin identity and release version even though each retains
its native marketplace schema. The legacy `nklisch/skilltap-skills` repository
is not a publisher to preserve after cutover.

This feature owns release automation and public documentation alignment; it
does not redefine the native plugin schemas, write harness caches, or archive
the sibling repository before the canonical publication has been verified.

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
- **Secondary publisher parity**: The active `../skills` marketplace remains a
  supported distribution path. Release checks fail on missing or mismatched
  skilltap plugin entries rather than silently allowing publisher drift.

<!-- Feature design will define the exact release checks and publication
artifacts. -->
