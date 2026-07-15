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
updated: 2026-07-15
---

# Resolve Cursor Admission

## Checkpoint

Realize exactly the Cursor boundary disposition through ordinary adapter ports.

An admitted result adds `cursor.rs`/`cursor_managed.rs`, an exact mutable
profile, verified skill roots and compatibility, both-scope managed projection,
a Cursor-owned `mcpServers` codec, and a bounded effective probe using the
current documented `agent mcp` commands. It edits only the attested global/project
`mcp.json` files, consumes the shared project-skill representation, and leaves
OAuth/extension registration native-owned.

An observe-only result registers only exact detection and bounded documented
observation under a verified-observe-only profile with no mutating ports. A
blocked result adds no adapter, fixture, constants, or registry entry.

## Acceptance evidence

- [x] No skill path or version is inferred from Cursor conventions; the adapter
      uses only the source-attested roots and documented `agent` command.
- [ ] Mutation/effective checks remain intentionally unresolved and are not
      represented by a codec, projection, lifecycle, or probe.
- [x] Observe-only/blocked outcomes expose no latent mutation path.
- [x] Unknown versions remain observe-only and no cache or OAuth state is read
      as desired configuration.

## Ordering

Depends on Cursor's boundary result and the shared file-managed contracts. It is
independent of Zoo and ZCode admission.

## Implementation notes

- Execution capability: direct work-item update only; no source or test changes.
- Boundary commit: `3b57655e`.
- Candidate gate commit: `8137cbd2`.
- Original mutation disposition: `blocked`.
- Relaxed registry disposition: `observe_only`.
- The mutation evidence remains binding: no exact compiled Cursor profile,
  skill/managed/native port, editor/CLI equivalence, effective reload,
  ownership-safe preservation/removal, auth/login/browser/editor path, or cache
  boundary was promoted.
- Added only a read-only Cursor adapter with documented `agent` version argv,
  strict one-line version decoding, source-attested skill/MCP observation roots,
  and registry metadata. Unknown versions remain no-write; no Cursor mutation
  port or effective probe exists.

## Verification

- Read the parent feature, candidate stories, boundary story, and gate story.
- Searched `crates/core/src`, `crates/harnesses/src`, and `crates/cli/src` for
  Cursor adapter/profile/port/registry artifacts; none were found.
- Confirmed registry/help/status/target-all tests include Cursor while native
  lifecycle and projection ports remain absent.

## Disposition

observe_only (registry admission); mutation remains blocked
