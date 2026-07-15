---
id: epic-expanded-harness-support-candidate-admission-zoo-admission
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-candidate-admission
depends_on: [epic-expanded-harness-support-candidate-admission-zoo-boundary, epic-expanded-harness-support-file-managed-contracts]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/zoocode-skills.md
  - .research/attestation/zoocode-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
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

- [x] Original mutation shape remains blocked: no exact profile, writer,
      projection, effective probe, or native lifecycle exists.
- [x] Relaxed registry shape is observe-only through the typed file-only
      contract; no guessed host argv or global-storage path exists.
- [x] Status reports unavailable host identity, global storage, and effective
      reload boundaries.

## Disposition rationale

**Original mutation disposition: Blocked.** This remains exactly as recorded by
the Zoo boundary checkpoint at commit `8b393752` under the original gate
`8137cbd2`: no compatible host, installed version, safe global storage path,
effective observation, preservation, ownership, removal, repeat, or full
isolation evidence was available. Source and distribution artifacts are not
mutation evidence.

**Relaxed registry disposition: ObserveOnly.** The new gate admits the
source-attested extension identity and safe documented `.roo`/`.agents` skill
roots plus project `.roo/mcp.json` as a narrow read-only target. The production
adapter has no binary, native lifecycle, skill projection, managed projection,
or effective probe. Global editor storage and effective reload remain explicit
unsupported surfaces.

## Implementation notes

- Added `crates/harnesses/src/adapters/zoo.rs` and one registry entry for
  observe-only documented declaration observation.
- Added no command argv, native lifecycle, skill/managed projection, editor
  storage writer, cache access, authentication, or effective probe.
- The adapter reads only the documented safe subset and reports unavailable
  host/global-storage/effective boundaries.

## Verification

- Preserved the boundary story's original blocked mutation evidence.
- Confirmed the relaxed gate requires only reliable identity plus safe documented
  read surfaces for registry admission and keeps exact compiled authority
  mandatory for mutation.
- Confirmed registry, help, status, `--target all`, bootstrap exclusion, and
  zero-native-write tests cover Zoo; all mutation ports remain absent.

## Ordering

Depends on Zoo's boundary result and the shared file-managed contracts. It is
independent of Cursor and ZCode admission.
