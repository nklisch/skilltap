---
id: epic-rust-control-plane-workspace-reset-distribution
kind: story
stage: done
tags: [infra]
parent: epic-rust-control-plane-workspace-reset
depends_on: [epic-rust-control-plane-workspace-reset-workspace]
release_binding: 3.0.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Preserve Release, Installer, and Homebrew Distribution

## Scope

Implement Unit 3 from the parent feature: build and attest four native Rust
release artifacts, remove npm publication, and align the curl installer,
local installer, Homebrew formula, and formula-update automation with the
unchanged artifact naming contract.

## Acceptance criteria

- [x] Release workflow builds and verifies Linux/macOS x64/arm64 binaries.
- [x] Installer and formula use the same four asset names as the workflow.
- [x] Shell scripts pass syntax checks and preserve configurable install paths.
- [x] No release job publishes or requires the retired npm packages.

## Implementation notes

- Files changed: `.github/workflows/release.yml`, `scripts/install-local.sh`,
  `homebrew-skilltap/Formula/skilltap.rb`, and
  `homebrew-skilltap/scripts/update-formula.sh`.
- Tests added: none; verified workflow YAML parsing, Ruby formula syntax, POSIX
  shell syntax, workspace tests, a real locked Rust release build and isolated
  local install, formula updates from representative checksums, rejection of
  invalid checksum input, and exact asset-name parity across release and
  Homebrew surfaces.
- Discrepancies from design: `install.sh` already derived the exact four asset
  names and honored `SKILLTAP_INSTALL`, so it required verification but no edit.
- Adjacent issues parked: none.
- Dispatch rationale: implemented as one bounded distribution surface with
  exclusive file ownership under the active autopilot wave.

## Review (2026-07-11)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Story verified by implement and integrated asset, workflow, shell,
Ruby, and Rust checks; fast-lane advance.
