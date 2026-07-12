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

<!-- Feature design will define the exact release checks and publication
artifacts. -->
