---
id: epic-expanded-harness-support-candidate-admission-zoo-admission
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-zoo-boundary, epic-expanded-harness-support-file-managed-contracts]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/zoocode-skills.md
  - .research/attestation/zoocode-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Resolve Zoo Code Admission

## Checkpoint

Realize exactly the disposition recorded by the Zoo boundary story after the
shared scope-aware managed/effective contract is available.

- **Admitted:** add distinct `zoo.rs` and `zoo_managed.rs` adapter modules, the
  exact mutable profile, skill root/compatibility port, Zoo-owned `mcpServers`
  codec and effective probe, managed projection in both scopes, fake profile,
  and one canonical registry entry.
- **Observe-only:** add only deterministic detection, verified-observe-only
  profile, bounded documented observation, optional read-only effective probe,
  and one registry entry. Return `None` for skill projection, managed projection,
  and native lifecycle.
- **Blocked:** add no adapter, path constants, fixture profile, or registry entry;
  preserve the blocker and prove absence in aggregate acceptance.

An admitted adapter consumes shared source checkout/projection, target-local
state, rollback, project-skill link, and acceptance machinery. It owns only Zoo
paths, version decoding, schema, precedence, and reload semantics. It never
writes editor extension storage or caches.

## Acceptance evidence

- [ ] Production shape exactly matches the boundary disposition.
- [ ] Admitted profile passes both scopes and both shared acceptance matrices,
      including effective reload, drift, ownership, removal, and repeat.
- [ ] Observe-only profile exposes no mutation capability or mutating port.
- [ ] Unknown versions are observe-only and runtime probes only narrow support.
- [ ] Blocked disposition leaves `zoo` absent from canonical registry/help and
      introduces no guessed path.

## Ordering

Depends on Zoo's boundary result and the shared file-managed contracts. It is
independent of Cursor and ZCode admission.
