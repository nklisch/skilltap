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
updated: 2026-07-14
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
- `observe_only` — canonical registry entry, verified-observe-only profile,
  deterministic status, and pre-write rejection for every mutation;
- `blocked` — no canonical registry/help/config mutation entry, no adapter/path
  constants, and explicit absence assertions.

Mixed-target scenarios select one candidate and prove desired inventory,
target-local state, journals, managed projections, files, and capability
profiles for both sibling candidates remain unchanged. Registry-derived help,
`harness list`/enablement, `--target all`, and first-party bootstrap reflect only
actual registered adapters; bootstrap stays Codex/Claude-only.

## Acceptance evidence

- [x] Cursor, Zoo, and ZCode each have exactly one `blocked` disposition matching
      their boundary and admission stories.
- [x] No admitted or observe-only acceptance fixture was added for a blocked
      target; the aggregate report records the absence of native evidence.
- [x] Every blocked target remains absent from the canonical registry, compiled
      help, config mutation, and `--target all` output, with no production
      adapter, path constant, profile, or port added.
- [x] Selecting a blocked candidate cannot alter or authorize Codex/Claude
      sibling configuration or state; first-party bootstrap remains Codex/Claude.
- [x] Compiled tests use isolated machine roots and leave operator HOME/XDG,
      caches, credentials, and native state outside the test boundary.
- [x] Workspace tests, all-feature Clippy with warnings denied, formatting, and
      `git diff --check` pass.

## Implementation notes

- Added the final blocked candidate report set to the dependency-neutral
  candidate matrix. Cursor, Zoo, and ZCode each produce exactly one `blocked`
  report with no invented native evidence or acceptance fixture.
- Added a harness-registry test that pins the canonical target set, proves all
  three candidate IDs are absent, and pins first-party bootstrap to Codex and
  Claude.
- Added compiled CLI coverage for registry-derived help, `--target all`, all
  relevant candidate mutation entry points, sibling config/state preservation,
  and non-first-party bootstrap rejection. All commands reject before mutation.
- No production adapter, path constant, profile, port, native fixture, or
  candidate target code was added.

## Verification

- Focused candidate report, harness detection/registry, and compiled CLI tests
  passed.
- `cargo check --workspace --all-features` passed.
- `cargo test --workspace --all-features` passed: 656 tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.

## Ordering

Final checkpoint after all three independent admission stories. Child stories
advance directly to done on green evidence; standard independent review occurs
once at the parent feature level.
