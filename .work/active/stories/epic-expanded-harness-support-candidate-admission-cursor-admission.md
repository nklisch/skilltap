---
id: epic-expanded-harness-support-candidate-admission-cursor-admission
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-cursor-boundary, epic-expanded-harness-support-file-managed-contracts]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/cursor-skills.md
  - .research/attestation/cursor-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Resolve Cursor Admission

## Checkpoint

Realize exactly the Cursor boundary disposition through ordinary adapter ports.

An admitted result adds `cursor.rs`/`cursor_managed.rs`, an exact mutable
profile, verified skill roots and compatibility, both-scope managed projection,
a Cursor-owned `mcpServers` codec, and a bounded effective probe using the
validated `cursor-agent mcp` commands. It edits only the attested global/project
`mcp.json` files, consumes the shared project-skill representation, and leaves
OAuth/extension registration native-owned.

An observe-only result registers only exact detection and bounded documented
observation under a verified-observe-only profile with no mutating ports. A
blocked result adds no adapter, fixture, constants, or registry entry.

## Acceptance evidence

- [ ] No skill path or version is inferred from Cursor conventions; every
      constant matches boundary evidence.
- [ ] Admitted MCP edits preserve unknown fields/unmanaged servers, block
      unowned same-name entries, and agree with fresh CLI server/tool state.
- [ ] Admitted complete skill trees, precedence, project representation,
      update/removal, target-local state, recovery, and repeats pass both shared
      matrices.
- [ ] Observe-only/blocked outcomes expose no latent mutation path.
- [ ] Unknown versions remain observe-only and no cache or OAuth state is read as
      desired configuration.

## Ordering

Depends on Cursor's boundary result and the shared file-managed contracts. It is
independent of Zoo and ZCode admission.

## Implementation notes

- Execution capability: direct work-item update only; no source or test changes.
- Boundary commit: `3b57655e`.
- Candidate gate commit: `8137cbd2`.
- Exact disposition: `blocked`.
- Rationale: the boundary story's `blocked` disposition is binding. It could not
  establish a safely isolated Cursor profile, exact installation identity,
  reproducible global/project skill roots, editor/CLI skill equivalence,
  effective MCP reload, ownership-safe preservation/removal, or a
  cache-independent observation boundary. The gate therefore provides neither
  the evidence required for `observe_only` nor the evidence required for
  `admitted`; under its disposition rules this candidate remains blocked.
- Verified production absence: no Cursor production adapter, Cursor production
  profile, Cursor-specific port, or Cursor registry entry exists. The canonical
  registry remains `codex`, `claude`, `gemini`, and `opencode`; no
  `crates/harnesses/src/*cursor*` file or Cursor adapter export exists. The only
  production-tree `cursor-v1` references are generic profile unit-test fixtures,
  not a registered Cursor target or adapter.
- No adapter, fixture, path constant, profile, port, registry entry, browser,
  authentication, login, or native Cursor state was added or accessed.

## Verification

- Read the parent feature, candidate stories, boundary story, and gate story.
- Searched `crates/core/src`, `crates/harnesses/src`, and `crates/cli/src` for
  Cursor adapter/profile/port/registry artifacts; none were found.
- Confirmed the canonical registry assertion remains exactly
  `["codex", "claude", "gemini", "opencode"]`.

## Disposition

blocked
