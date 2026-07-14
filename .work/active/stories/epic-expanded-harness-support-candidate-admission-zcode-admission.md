---
id: epic-expanded-harness-support-candidate-admission-zcode-admission
kind: story
stage: implementing
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
updated: 2026-07-14
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
- [ ] Observe-only/blocked outcomes cannot reach managed or native execution.
- [ ] Import databases, editor caches, and credentials remain untouched.

## Ordering

Depends on ZCode's boundary result and the shared file-managed contracts. It is
independent of Zoo and Cursor admission.
