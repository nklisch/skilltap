---
id: epic-expanded-harness-support-candidate-admission-zoo-admission
kind: story
stage: done
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

- [x] Production shape exactly matches the boundary disposition: `blocked`.
- [x] No admitted profile or shared acceptance matrix was added because the
      boundary did not establish an admissible or observe-only runtime.
- [x] No observe-only profile or mutating port was added.
- [x] No version probe was added; the boundary found no compatible host or
      installed extension from which to derive one.
- [x] Blocked disposition leaves `zoo` absent from adapters, profiles, ports,
      path constants, canonical registry/help, and fixtures.

## Disposition rationale

**Blocked**, exactly as recorded by the Zoo boundary checkpoint at commit
`8b393752` under the candidate-admission gate established at commit `8137cbd2`.
The boundary's exact rationale is that Zoo is an editor extension, no compatible
host or Zoo executable was available, no installed extension identity/version or
safe deterministic read-only effective-state observation could be obtained, and
all native skill/MCP discovery, precedence, reload, preservation, ownership,
removal, repeat, and full isolation checks therefore remained unavailable under
the mandated non-UI isolation. Source and distribution artifacts were not
promoted to runtime evidence. Consequently this story intentionally adds no
adapter, profile, port, path constant, fixture, or registry/help entry.

## Implementation notes

- Files changed: this story only.
- No Zoo adapter/profile/port/registry entry was added.
- No production or test-support source was changed.
- No candidate integration test was added because the boundary explicitly
  forbids one until native roots and processes can be safely isolated.

## Verification

- Confirmed the boundary story at `8b393752` records `**blocked**` and names
  every missing admission check.
- Confirmed the shared gate at `8137cbd2` provides the disposition contract and
  treats incomplete observation as `Blocked`.
- Confirmed the production tree contains no Zoo adapter, profile, port, path
  constant, or registry entry; the only `zoo` source hit is the gate's
  test-support matrix fixture.
- Confirmed no Zoo-named adapter/profile/port file or candidate integration test
  exists.

## Ordering

Depends on Zoo's boundary result and the shared file-managed contracts. It is
independent of Cursor and ZCode admission.
