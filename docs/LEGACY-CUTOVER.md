# Legacy skilltap publisher cutover

This record describes the one-way retirement of the public
`nklisch/skilltap-skills` repository. The canonical implementation and release
source is this `nklisch/skilltap` repository.

## Retirement targets

After the canonical release evidence gate passes, retire the old skilltap
surfaces in `nklisch/skilltap-skills`, including:

- the former skilltap orientation/management skill;
- the former `claude-code-marketplace` guidance surface when it is the
  superseded skilltap distribution; and
- README/install links that direct users to the retired skilltap publisher.

Do not retire unrelated skills or content in that repository without a
separate owner decision. This record is not a compatibility layer and does
not preserve the old TypeScript implementation as active guidance.

## Canonical replacement

Users should install the first-party plugin from the native Claude Code or
Codex marketplace, or use the online installer:

```sh
curl -fsSL https://skilltap.dev/install.sh | sh
skilltap bootstrap
```

The canonical plugin contains the complete `skilltap` skill directory with
top-level `SKILL.md` and sibling references. `skilltap bootstrap` verifies the
binary separately from each harness setup and reports unsupported native paths
as explicit next actions. `skilltap --help` and leaf help remain authoritative
for syntax.

## Active sibling publisher

The local `../skills` repository is active and is not the retirement target.
Its marketplace must retain one `skilltap` entry that points directly at this
repository's canonical `plugin/` subdirectory. Run the read-only parity check
with `SKILLTAP_SKILLS_MARKETPLACE` when that checkout is available. Never
archive, delete, or rewrite `../skills` as part of this cutover.

## Operator-gated checklist

The following actions require a deliberate operator or external repository
workflow after `scripts/verify-cutover.sh` succeeds:

1. Verify a published canonical release, website installer, and binary
   bootstrap on a clean supported macOS or Linux environment.
2. Verify the native marketplace plugin loads the complete skill and that the
   implicit skill is available to the intended agents.
3. Run the sibling pointer parity check against the active `../skills`
   checkout; repair its marketplace entry in that repository if needed.
4. In `nklisch/skilltap-skills`, remove the retired skilltap/marketplace files
   and update its README/deprecation notice, preserving unrelated content.
5. Archive the public legacy repository through its repository-owner controls
   only after the prior steps are recorded. Do not claim archival from a local
   checkout or from this document alone.

The checklist is idempotent: rerunning evidence checks is safe, and each
external deletion/archive action must be confirmed against the current remote
state before execution. This repository never executes those destructive
remote actions implicitly.
