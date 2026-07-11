---
id: epic-standalone-skill-lifecycle
kind: epic
stage: review
tags: []
parent: null
depends_on: [epic-reconciliation-execution]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Standalone Skill Lifecycle

## Brief

Deliver complete-directory standalone skill management from explicit local or
Git sources. The capability validates a top-level `SKILL.md`, preserves every
sibling resource, fingerprints the whole tree, records provenance and resolved
Git revisions, and installs the canonical managed representation at the
appropriate global or project scope.

Harness projections use native paths, links, or copies only where required.
The lifecycle includes compatibility evidence, drift-safe removal and update,
Git SHA comparison, and pins without recursively discovering skills or
separating `SKILL.md` from its directory.

## Foundation references

- `docs/SPEC.md` — Standalone Skill Model, Standalone Skill Lifecycle, Skill Compatibility
- `docs/ARCH.md` — Standalone Skills, Compatibility Analysis, Updates
- `docs/HARNESS-CONTRACTS.md` — Standalone Skill Contract, Version and Update Contract
- `docs/UX.md` — Standalone Skills, Skill Updates

## Design decisions

- **How are local-directory skills installed?** Copy a snapshot of the entire
  source directory into the canonical managed location. Source changes become
  detectable drift or an available explicit update; they do not alter the
  installed skill live. A source already occupying the canonical location is
  handled through adoption rather than a self-referential install.
- **What does `--name` mean?** Treat it as an expected-name assertion. The
  resolved `SKILL.md` name must equal the supplied value; skilltap never creates
  an alias or rewrites the skill directory or frontmatter.
- **Must every managed skill strictly conform to Agent Skills?** Track strict
  conformance separately from verified harness loadability. A nonconforming
  skill may install only when every selected harness is known to load that
  exact shape, and the plan warns about the violation and requires explicit
  foreground acknowledgment. If any target cannot load it or loadability is
  unknown, the operation remains blocked.
- **Does this epic require UI mockups?** No. Skill lifecycle and compatibility
  decisions are non-interactive CLI and JSON surfaces.

## Anticipated child features

- Explicit local and Git skill source resolution
- Whole-directory validation and deterministic fingerprinting
- Canonical `.agents/skills/` storage and harness projections
- Evidence-bearing compatibility classification
- Install, list, remove, and drift handling
- Ref-to-SHA update resolution, pins, and safe replacement

<!-- The design pass on each child feature will fill in real specifics. -->

## Decomposition

The lifecycle is split so source resolution and whole-directory integrity are
pure and reusable, while storage/projection and command orchestration remain
behind the existing reconciliation ports.

## Children complete

All source, tree, storage, compatibility, and command features are done. The
realized lifecycle persists Git refs/SHA and subdirectories, filters repository
metadata from complete skill trees, publishes canonical `.agents/skills`
artifacts plus required Claude projections, and handles update-all, drift, and
ownership safely.

### Child features

1. `epic-standalone-skill-lifecycle-source` — resolve explicit local/Git
   sources, requested refs, subdirectories, and expected names without scanning
   for skills — depends on `[]`.
2. `epic-standalone-skill-lifecycle-tree` — validate a complete skill
   directory, require top-level `SKILL.md`, preserve siblings, and fingerprint
   deterministic tree contents — depends on
   `[epic-standalone-skill-lifecycle-source]`.
3. `epic-standalone-skill-lifecycle-storage` — publish immutable canonical
   `.agents/skills/<name>/` trees and managed records at global/project scope,
   with no-clobber backups and ownership checks — depends on
   `[epic-standalone-skill-lifecycle-tree]`.
4. `epic-standalone-skill-lifecycle-compatibility` — parse frontmatter and
   classify Codex/Claude loadability, warnings, and exact partial consequences
   without inventing faithful equivalence — depends on
   `[epic-standalone-skill-lifecycle-tree]`.
5. `epic-standalone-skill-lifecycle-commands` — compose install/list/remove/
   update with Git SHA comparison, pins, projections, selectors, and
   idempotent CLI output — depends on
   `[epic-standalone-skill-lifecycle-storage,
   epic-standalone-skill-lifecycle-compatibility]`.

## Design review

### Verdict

Approved for implementation.

### Notes

The canonical managed form is always the complete directory. A source tree is
never live-linked into managed storage, and no command recursively discovers
skills. Native harness links/copies remain projections owned by later command
and harness adapters.
