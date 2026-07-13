---
id: story-skilltap-plugin-distribution-release-install-surfaces
kind: story
stage: done
tags: [content, testing]
parent: epic-skilltap-plugin-distribution-release
depends_on: [story-skilltap-plugin-distribution-release-contract]
release_binding: 3.0.2
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Align website, Homebrew, and secondary marketplace installation surfaces

Rewrite website/README installation and update guidance so marketplace plugin
installation and the online installer are equal first-class paths. Keep
Homebrew's binary-only story explicit and validate the active `../skills`
marketplace entry points directly at this repository's canonical `plugin/`
subdirectory without modifying or archiving that active repository.

Acceptance criteria:

- Public docs explain bootstrap, binary/harness result separation, opt-out,
  same-major safe updates, and explicit major acknowledgment consistently.
- Homebrew docs/formula do not claim to install harness plugins automatically.
- An offline parity check fails when the sibling marketplace source pointer or
  canonical plugin identity drifts.

## Implementation notes
- Execution capability: standard; public installation prose and parity checks.
- Review weight: standard (autopilot caller policy).
- Files changed: `README.md`, `website/guide/getting-started.md`, `scripts/verify-install-surfaces.sh`, `.github/workflows/ci.yml`.
- Tests added: offline public-surface parity checks for marketplace/installer/bootstrap/update/Homebrew wording; optional read-only sibling pointer validation via `SKILLTAP_SKILLS_MARKETPLACE`.
- Discrepancies from design: active sibling checkout is not modified; pointer validation is opt-in when a parity checkout is supplied.
- Adjacent issues parked: none.

## Review (2026-07-12)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: none
**Nits**: the active `../skills` checkout remains intentionally untouched;
set `SKILLTAP_SKILLS_MARKETPLACE` in an external parity checkout to enforce its
direct canonical pointer.

**Notes**: Fast substrate review at standard weight. README and website make
marketplace and one-line installation equal paths, explain bootstrap result
separation and update policy, and keep Homebrew binary-only. The offline
surface script passes and performs only an optional read-only sibling check.
