---
id: epic-expanded-harness-support-candidate-admission-acceptance
kind: story
stage: done
tags: [testing]
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-zoo-admission, epic-expanded-harness-support-candidate-admission-zcode-admission, epic-expanded-harness-support-candidate-admission-cursor-admission]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/cursor-skills.md
  - .research/attestation/cursor-mcp.md
  - .research/attestation/zoocode-skills.md
  - .research/attestation/zoocode-mcp.md
  - .research/attestation/zcode-skills.md
  - .research/attestation/zcode-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Verify Candidate Dispositions and Isolation

## Checkpoint

Prove the aggregate feature tells the truth for all three target-local outcomes
without requiring all three candidates to be admitted.

Extend candidate reports, harness detection/contract tests, production-aware
managed acceptance runners, and compiled CLI coverage only for the dispositions
actually established:

- `admitted` — canonical registry entry, exact mutable profile, both shared
  matrices, and complete lifecycle/effective evidence;
- `observe_only` — canonical registry entry, verified-observe-only or typed
  file-only profile, deterministic status, and no mutation/effective ports;
- `blocked` — no canonical registry/help/config mutation entry, no adapter/path
  constants, and explicit absence assertions when the relaxed read gate fails.

Mixed-target scenarios select one candidate and prove desired inventory,
target-local state, journals, managed projections, files, and capability
profiles for both sibling candidates remain unchanged. Registry-derived help,
`harness list`/enablement, `--target all`, and first-party bootstrap reflect only
actual registered adapters; bootstrap stays Codex/Claude-only.

## Acceptance evidence

- [x] Cursor, Zoo, and ZCode each have exactly one `observe_only` registry
      disposition, while their original blocked mutation evidence remains in
      the boundary/admission stories.
- [x] Registry-derived help, harness list/status, `--target all`, and target
      selection include all three without adding mutation authority.
- [x] All three have no native lifecycle, skill, managed projection, or
      effective-state port; file-only Zoo/ZCode entries have no binary policy
      and no guessed argv.
- [x] Cursor unknown versions remain no-write; Zoo/ZCode use only the typed
      read-only/file-only contract; project skill remains unsupported for ZCode.
- [x] First-party bootstrap remains Codex/Claude-only and candidate commands
      leave isolated native/editor/cache/auth state byte-for-byte unchanged.
- [x] Compiled tests use isolated machine roots and leave operator HOME/XDG,
      caches, credentials, and native state outside the test boundary.
- [x] Workspace tests, all-feature Clippy with warnings denied, formatting, and
      `git diff --check` pass.

## Implementation notes

- Replaced the blocked-only aggregate report with three concrete observe-only
  reports and a relaxed gate requiring reliable identity plus safe documented
  reads; exact compiled mutation authority remains mandatory.
- Added distinct Cursor, Zoo, and ZCode adapters. Cursor exposes only bounded
  `agent --version` plus documented observation; Zoo and ZCode use the typed
  file-only contract. None exposes mutation or effective-state ports.
- Added compiled CLI coverage for registry-derived help, status, `--target all`,
  first-party bootstrap exclusion, candidate target selection, and zero-native-
  write behavior. Existing Codex/Claude sibling state remains isolated.

## Verification

- Focused candidate report, harness detection/registry, and compiled CLI tests
  passed.
- `cargo check --workspace --all-features` passed.
- Workspace verification is rerun for the amended registry and recorded at
  the parent feature after the final workspace ladder.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.

## Ordering

Final checkpoint after all three amended admission stories. Child stories
advance directly to done on green evidence; the parent feature is now at
`stage: review` and must receive its separate feature-level review.
