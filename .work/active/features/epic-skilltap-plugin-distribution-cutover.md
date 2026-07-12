---
id: epic-skilltap-plugin-distribution-cutover
kind: feature
stage: review
tags: [infra, content, cleanup]
parent: epic-skilltap-plugin-distribution
depends_on: [epic-skilltap-plugin-distribution-release]
release_binding: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Retire the Legacy Skilltap Skills Publisher

## Brief

Complete the one-way publication cutover from the public
`nklisch/skilltap-skills` repository to the canonical skilltap repository.
Remove or retire its old skilltap-adjacent guidance, including
`claude-code-marketplace` where it is superseded, add an explicit
deprecation/archive record, and verify that users have a working canonical
plugin and binary path before the old publisher disappears.

This feature tracks the cross-repository handoff and evidence; it does not
create a compatibility layer or preserve the legacy repository as a second
source of truth. The active local `../skills` repository remains a supported
second publisher and must retain its synchronized skilltap entry. Any
remaining non-skilltap content in `skilltap-skills` is outside this feature's
product scope, even if repository archival requires an operator-level decision
about its future.

## Epic context

- Parent epic: `epic-skilltap-plugin-distribution`
- Position in epic: terminal cutover consumer; it runs only after a verified
  canonical release.

## Foundation references

- `docs/VISION.md` — canonical publisher boundary
- `docs/SPEC.md` — Self-Hosted Plugin Distribution
- `docs/ARCH.md` — Plugin Publication Boundary
- `https://github.com/nklisch/skilltap-skills` — legacy public repository
- `https://github.com/nklisch/skilltap-skills/blob/main/README.md` — current
  legacy skilltap distribution surface
- `https://github.com/nklisch/skilltap-skills/tree/main/.agents/skills` —
  superseded guidance locations

## Design decisions

- **Retirement target**: Only the public `nklisch/skilltap-skills` repository is
  retired. The active sibling `../skills` repository remains fully supported as
  a synchronized publisher and is not archived.
- **Cutover gate**: Retire the legacy repository only after the canonical
  plugin, website installer, binary bootstrap, and implicit skill are verified
  as usable.

<!-- Feature design will define the cutover checklist and external-repository
handoff without changing skilltap's native resource semantics. -->

## Architectural choice

Model cutover as an evidence-gated, one-way handoff with no compatibility
shim. This repository records the canonical plugin and binary paths, verifies
the active sibling pointer without mutating it, and emits an operator-ready
retirement/archive record for the external `nklisch/skilltap-skills` repository.
Only after canonical release evidence is complete may the legacy repository's
skilltap surfaces be deleted or the repository archived through its own
operator controls.

Alternative approaches considered:

1. Keep a redirect/compatibility plugin in the legacy repository. This creates
   a second source of truth and prolongs ambiguity.
2. Delete the legacy repository immediately. This risks stranding users before
   the canonical plugin, installer, and implicit skill are verified.
3. Chosen: evidence-gated handoff with explicit deprecation text and a
   destructive operator checklist; no runtime compatibility layer.

The riskiest unit is the external retirement handoff because this checkout
cannot safely archive or delete another public repository. It must produce
precise, idempotent instructions and evidence while leaving the active
`../skills` publisher untouched.

## Implementation Units

### Unit 1: Canonical cutover verification

**Files**: `scripts/verify-cutover.sh`, release/install fixtures
**Story**: `story-skilltap-plugin-distribution-cutover-canonical-verification`

Verify the published canonical plugin source, native channel manifests,
website installer, binary bootstrap, and implicit agent skill as one usable
path. The script remains offline by default and accepts explicit release
fixtures/URLs for operator verification; it never writes harness caches.

**Acceptance criteria**:

- [ ] Canonical package validation, installer checks, and bootstrap result
      checks pass before any legacy retirement step.
- [ ] The complete `skilltap` skill directory is present and implicitly
      available through both native package trees.
- [ ] A failed or missing release evidence gate exits nonzero with the exact
      missing next action.

### Unit 2: Legacy skill surface inventory and deprecation record

**Files**: `docs/LEGACY-CUTOVER.md`, `scripts/verify-cutover.sh`
**Story**: `story-skilltap-plugin-distribution-cutover-legacy-record`

Record the superseded `skilltap` and `claude-code-marketplace` surfaces,
canonical replacements, user-facing migration sequence, and destructive
deletion/archive checklist. Keep the record current-state oriented and avoid
retaining old TypeScript implementation guidance in active docs.

**Acceptance criteria**:

- [ ] Legacy repository and old skill paths are named explicitly, with the
      canonical plugin/website/bootstrap replacement for each.
- [ ] The record says the active `../skills` repository is not the retirement
      target and must remain a direct canonical publisher.
- [ ] Deletion/archive steps are idempotent, operator-confirmed, and gated on
      canonical release evidence; this repo performs no external destructive
      operation implicitly.

### Unit 3: Active sibling publisher parity handoff

**Files**: `scripts/verify-install-surfaces.sh`, `docs/LEGACY-CUTOVER.md`
**Story**: `story-skilltap-plugin-distribution-cutover-sibling-parity`

Provide a read-only check for the active `../skills` marketplace entry. It must
point directly at this repository's `plugin/` subtree and preserve one public
identity/version source. The check accepts an explicit sibling checkout path
for CI/operator use and skips safely when the sibling checkout is unavailable.

**Acceptance criteria**:

- [ ] A valid sibling catalog entry resolves directly to the canonical repo
      plugin path and `skilltap` identity.
- [ ] A missing, copied, or wrong-repository entry produces a remediation
      message without editing the sibling checkout.
- [ ] No archive/delete action can target `../skills`.

## Implementation Order

1. `story-skilltap-plugin-distribution-cutover-canonical-verification`
2. `story-skilltap-plugin-distribution-cutover-legacy-record` and
   `story-skilltap-plugin-distribution-cutover-sibling-parity` in parallel
3. Feature review records the external operator handoff; archive/delete remains
   an explicit user-controlled action outside this workspace.

## Testing

- Offline package, installer, and bootstrap fixtures provide the canonical
  evidence gate.
- Shell tests exercise missing release evidence and malformed legacy/sibling
  pointers without network or external writes.
- Documentation review checks replacement links, retirement target names, and
  the explicit exclusion of active `../skills`.

## Risks

- External repository archival authority may be unavailable in this workspace.
  Preserve a handoff record and do not claim archival until an operator or
  external workflow confirms it.
- Users may still reference old raw skill URLs. The deprecation record must
  point to the canonical plugin and bootstrap path without maintaining a
  compatibility implementation.
- A sibling pointer can drift independently. The parity check should be
  explicit and read-only, with no automatic cross-repository mutation.

## Design decisions

- **Destruction boundary**: no automatic external delete/archive; operator
  follows the evidence-gated checklist after release verification.
- **Retirement scope**: only public `nklisch/skilltap-skills`; active
  `../skills` remains supported and is never archived by cutover.
- **Retention**: delete-refs applies to agile work records; the deprecation
  record keeps only current replacement/handoff facts, with git history as the
  audit trail.

## Children complete

The three cutover stories are complete: canonical evidence gate, legacy
deprecation/archive handoff record, and read-only active sibling pointer parity.
