---
id: epic-expanded-harness-support-candidate-admission-acceptance
kind: story
stage: implementing
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

- [ ] Cursor, Zoo, and ZCode each have exactly one disposition matching their
      boundary and admission stories.
- [ ] Every admitted target passes exact detection, both scopes, whole skills,
      MCP schema/precedence/secrets, effective reload, trust/policy health where
      relevant, drift, ownership, update/removal, pending recovery, partial and
      required compatibility, and immediate-repeat no-change.
- [ ] Every observe-only target is inspectable but has no mutation capabilities,
      skill/managed projection port, native lifecycle, or write operation.
- [ ] Every blocked target remains absent with no guessed surface.
- [ ] One candidate's success cannot alter or authorize either sibling.
- [ ] Operator HOME/XDG/editor roots, extension storage, caches, and credentials
      remain byte-for-byte untouched by tests.
- [ ] Workspace tests, all-feature Clippy with warnings denied, formatting, and
      `git diff --check` pass before the feature enters review.

## Ordering

Final checkpoint after all three independent admission stories. Child stories
advance directly to done on green evidence; standard independent review occurs
once at the parent feature level.
