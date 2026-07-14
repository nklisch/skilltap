---
id: epic-expanded-harness-support-project-skill-links
kind: feature
stage: drafting
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-standalone-skill-lifecycle, epic-expanded-harness-support-registry]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
  - .research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md
research_origin: operator-request-2026-07-14
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Validate and Link Project-Local Skills

## Brief

Make one project-local portable skill tree authoritative across every selected
harness. The canonical complete skill directory remains
`<project>/.agents/skills/<name>/`; when a harness loads project skills from a
different native root, skilltap projects that skill as a per-skill relative
symlink back to the canonical directory. A harness that already consumes the
canonical location requires no projection.

Validate the canonical complete directory before planning links and report two
separate results: strict Agent Skills conformance and target-specific
loadability/compatibility. Validation covers the exact top-level `SKILL.md`,
portable frontmatter and name/directory invariants, complete-tree integrity,
and target-specific evidence already modeled by the standalone skill
lifecycle. A malformed or incompatible canonical skill is reported without
creating or repairing native links.

This feature extends the existing explicit project skill lifecycle; it does not
implicitly take ownership of every directory it can find. Skills become managed
through the existing install, adoption, or desired-inventory paths. Status,
plan, sync, update, and removal then observe and reconcile the canonical tree
and its selected target links as one resource.

## Strategic decisions

- **Canonical location:** `<project>/.agents/skills/<name>/` is the single
  project source of truth because it is the broadest portable convention and a
  native Codex load path.
- **Projection form:** use a relative symlink for each skill directory, not a
  copied tree and not a symlink for the whole harness skill root. Per-skill
  links preserve unmanaged and harness-specific sibling skills.
- **Selection and ownership:** preserve skilltap's explicit lifecycle. Merely
  observing an unmanaged canonical skill does not authorize mutation; adoption
  or installation establishes desired state and ownership.
- **Validation model:** keep strict format conformance distinct from observed
  target loadability. Client tolerance may produce a warning, but it never
  turns a nonconforming skill into a conforming one.
- **Conflict policy:** an existing regular file, directory, absolute symlink,
  or symlink to another target at a native destination is drift or an ownership
  conflict. `sync` does not overwrite it silently. A missing, broken, or
  incorrect skilltap-owned relative link is repairable through the normal plan
  and revalidated execution path.
- **Scope boundary:** this feature covers project scope. Global managed-skill
  representation remains unchanged unless separately scoped.

## Simplification opportunity

Stop publishing and fingerprinting duplicate complete trees in
`.agents/skills/` and harness-native project roots. Reuse the existing relative
symlink path logic, no-follow filesystem inspection, target registry, and
managed-skill execution boundary so canonical content validation, ownership,
and drift have one source of truth. Do not add a second project-skill registry,
manifest, or discovery command.

## Foundation references

- `docs/SPEC.md` — project scope, standalone skill model and lifecycle,
  compatibility, ownership, and symlink safety.
- `docs/ARCH.md` — standalone skill source of truth, adapter projection port,
  observation, and revalidated apply flow.
- `docs/HARNESS-CONTRACTS.md` — canonical `.agents/skills` placement, native
  project skill roots, whole-directory loading, and target compatibility.
- `.research/analysis/briefs/current-agent-extension-standards.md` — portable
  skill boundary and adapter projection posture.

The foundation already permits canonical `.agents/skills` trees and native
adapter links, while retaining copies for other scopes or independently scoped
fallbacks. This feature makes the project-scoped representation precise without
changing the standing product direction, so no foundation document is rolled
forward at scope time.

## Acceptance direction

- A valid managed project skill at `.agents/skills/<name>` produces one
  relative per-skill symlink in every selected distinct native skill root; the
  relative target resolves exactly to the canonical directory.
- Codex and any future harness whose native destination equals the canonical
  path produce no redundant operation.
- Correct links are immediate-repeat no-ops. Missing or skilltap-owned broken
  links are repaired; unmanaged or divergent destinations are reported and
  preserved.
- Install/update publishes and validates the canonical tree before dependent
  link operations. Remove deletes only proven skilltap-owned links and removes
  the canonical tree only when the selected resource removal is safe.
- Status and JSON output distinguish canonical format errors, target
  incompatibility, missing links, broken links, divergent links, and unmanaged
  destination conflicts without following a link for ownership decisions.
- Planning and application use the target registry rather than harness-id
  branching, revalidate link identity under the configuration lock, and remain
  idempotent on macOS and Linux.
- Isolated integration coverage exercises multiple harness roots, nested
  project paths, relative target calculation, complete skill siblings,
  conflicts, repair, removal, and an immediate repeated sync with zero changes.

## Dependency integration

This feature is the project-skill projection contract consumed by each pending
expanded-harness adapter family. Those features retain their native load-path
and effective-state responsibilities but do not invent their own copied-tree or
link reconciliation behavior.
