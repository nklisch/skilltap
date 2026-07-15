---
id: epic-expanded-harness-support-candidate-admission-zcode-admission
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-zcode-boundary, epic-expanded-harness-support-file-managed-contracts]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/zcode-skills.md
  - .research/attestation/zcode-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Resolve ZCode Admission

## Checkpoint

Realize exactly the ZCode boundary disposition using ordinary registry and
managed projection contracts.

An admitted result adds distinct `zcode.rs`/`zcode_managed.rs` modules, exact
mutable profile, verified skill roots, target-owned MCP codec/effective probe,
both-scope managed projection, fake profile, and registry entry. Project skills
flow through `project_skill_projection`; adapter code does not invoke or
reimplement ZCode's copy/symlink import lifecycle.

An observe-only result adds detection and documented bounded observation with a
verified-observe-only profile but no mutating ports. A blocked result adds no
production adapter, constants, fixture, or registry entry.

## Acceptance evidence

- [ ] Exact native files, version, copy/symlink semantics, enablement, and
      precedence are sourced only from the boundary evidence.
- [ ] Admitted codec preserves unknown/unmanaged entries and rejects same-name
      conflicts, malformed containers, unsupported transport/auth, and secret
      acquisition.
- [ ] Admitted lifecycle passes complete-skill, effective reload, ownership,
      update/removal, recovery, target-state, and repeat acceptance in both
      scopes.
- [x] Observe-only/blocked outcomes cannot reach managed or native execution.
- [ ] Import databases, editor caches, and credentials remain untouched.

## Disposition rationale

**Original mutation disposition: Blocked**, exactly as recorded by the boundary
at commit `0b56a448` under the original gate `8137cbd2`. The exact project skill
root, deterministic installed identity, redirectable profile, effective reload,
preservation, ownership, and repeat evidence remain absent, so no mutation
profile or writer is authorized.

**Relaxed registry disposition: ObserveOnly.** The typed file-only adapter reads
only the exact documented global skill and user/workspace MCP declaration files.
It has no executable, default command, native lifecycle, skill projection,
managed projection, or effective probe. Project skill observation remains
unsupported and all unresolved boundaries are reported.

## Implementation notes

- Added `crates/harnesses/src/adapters/zcode.rs` and one registry entry through
  the typed file-only read-only contract.
- Added no executable argv, default command, mutation port, import UI path,
  cache/database access, browser, authentication, login, or native writer.
- Project skill remains unsupported exactly as the boundary evidence requires.

## Verification

- Preserved and re-read the original boundary's blocked mutation evidence.
- Confirmed the relaxed gate uses reliable identity plus safe documented reads
  for registry admission while exact compiled authority remains mandatory for
  writes.
- Confirmed registry, help, status, `--target all`, bootstrap exclusion, and
  zero-native-write tests cover ZCode; no mutation port or project skill root
  exists.

## Ordering

Depends on ZCode's boundary result and the shared file-managed contracts. It is
independent of Zoo and Cursor admission.

## Disposition

observe_only (registry admission); mutation remains blocked
