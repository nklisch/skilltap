---
id: epic-expanded-harness-support-file-managed-acceptance
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-file-managed
depends_on: [epic-expanded-harness-support-file-managed-gemini, epic-expanded-harness-support-file-managed-opencode, epic-expanded-harness-support-file-managed-kiro]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Prove Integrated File-Managed Adapter Acceptance

## Checkpoint

Integrate the three completed adapters through the canonical registry, CLI,
status, project-skill links, shared managed lifecycle, and both reusable
acceptance matrices. This is integrated evidence, not a new adapter abstraction.

## Registry and output contract

- Canonical order is `codex`, `claude`, `gemini`, `opencode`, `kiro`.
- Help, `harness enable/list`, config policy, enabled resolution, status labels,
  and `--target all` derive from that registry.
- First-party bootstrap remains exactly Codex and Claude.
- Plain and JSON outcomes distinguish declared, effective, untrusted/unverified,
  drifted, and conflict states from one normalized result.

## Acceptance evidence

- Add data-driven `FakeHarnessProfile::{gemini, opencode, kiro}` and managed
  projection profiles; no target-id layout match remains.
- For each adapter, prove exact detection/profile behavior, global and project
  complete skill roots, project precedence, MCP merge and secret boundaries,
  bounded status/reload, optional versus required compatibility, drift,
  ownership, pending recovery, rollback, target-local state, removal, and
  immediate-repeat no change.
- Prove Gemini/OpenCode canonical project no-link behavior and Kiro relative-link
  behavior through compiled CLI tests.
- Assert unknown versions cannot reach managed apply.
- Assert native Gemini extensions, OpenCode plugin/Bun cache, and Kiro Powers/IDE
  paths stay byte-for-byte untouched.
- Use only test-support-owned HOME, XDG, KIRO_HOME, workspaces, sources, and fake
  executables.
- Run workspace tests, all-feature Clippy with warnings denied, formatting, and
  `git diff --check` before advancing the parent to feature review.

## Ordering

This checkpoint depends on all three adapter checkpoints. Child stories advance
directly to done on green verification; only the parent feature receives the
standard one-pass independent implementation review.
