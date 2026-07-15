---
id: epic-expanded-harness-support-file-managed-acceptance
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-file-managed
depends_on: [epic-expanded-harness-support-file-managed-gemini, epic-expanded-harness-support-file-managed-opencode, epic-expanded-harness-support-file-managed-kiro]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Prove Integrated File-Managed Adapter Acceptance

## Checkpoint

Integrate the three completed adapters through the canonical registry, CLI,
status, project-skill links, shared managed lifecycle, and both reusable
acceptance matrices. This is integrated evidence, not a new adapter abstraction.

## Registry and output contract

- The existing registry order is preserved and Kiro is appended after OpenCode:
  `codex`, `claude`, `droid`, `gemini`, `qwen`, `opencode`, `kiro`, `pi`.
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
  bounded status/reload where attested, optional versus required compatibility,
  drift, ownership, pending recovery, rollback, target-local state, removal,
  and immediate-repeat no change. Kiro is declaration-managed: it invokes no
  effective probe and remains effective-unverified.
- Prove Gemini/OpenCode canonical project no-link behavior and Kiro relative-link
  behavior through compiled CLI tests.
- Assert unknown versions cannot reach managed apply.
- Assert native Gemini extensions, OpenCode plugin/Bun cache, and Kiro Powers/IDE
  paths stay byte-for-byte untouched.
- Use only test-support-owned HOME, XDG, KIRO_HOME, workspaces, sources, and fake
  executables.
- Run workspace tests, all-feature Clippy with warnings denied, formatting, and
  `git diff --check` before advancing the parent to feature review.

## Implementation evidence

- Registered and exported Kiro with its exact 2.12.2 declaration-managed
  profile, scoped `ManagedDeclarationContract`, and no effective probe. The
  existing managed lifecycle now admits exact Unverified foreground plans so
  the shared acknowledgment executor, rather than an adapter-local gate,
  controls Kiro writes.
- Added isolated compiled Kiro acceptance for global/project plugin
  declarations, no-ack blocking, `--yes` application, effective-unverified
  status, daemon target no-write behavior, exact repeat no-op, project skill
  links, KIRO_HOME isolation, Power/cache/login avoidance, and unknown/adjacent
  version no-write behavior.
- Added data-driven Gemini/OpenCode/Kiro fixture layouts and managed profile
  descriptors. The reusable file-managed and production-aware managed
  acceptance matrices run these profiles alongside the existing Codex/Claude,
  Factory, Qwen, and fake-managed regressions.

## Verification

- `cargo test -p skilltap-harnesses --all-targets` — passed.
- `cargo test -p skilltap-test-support --all-targets` — passed.
- `cargo test -p skilltap --lib managed_projection_profiles_pass_the_shared_acceptance_matrix_repeatedly` — passed.
- `cargo test -p skilltap --test compiled_binary kiro_` — 2 passed.
- Final workspace/all-target, strict Clippy, formatting, and diff checks are the
  closing verification for this checkpoint.

## Ordering

This checkpoint depends on all three adapter checkpoints. Child stories advance
directly to done on green verification; only the parent feature receives the
standard one-pass independent implementation review.
