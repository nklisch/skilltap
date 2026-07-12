---
id: gate-security-workflow-action-pinning
kind: story
stage: done
tags: [security]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: security
created: 2026-07-12
updated: 2026-07-12
---

# Pin release and deployment actions to immutable revisions

## Severity

High

## Domain

CI and supply-chain security

## Location

- `.github/workflows/release.yml:14-17,26,56,79,83,93,102,118,124,143`
- `.github/workflows/ci.yml:14,37,54,56`
- `.github/workflows/deploy.yml:25,29,39,45,58`

## Evidence

Release, CI, and deployment workflows reference mutable third-party action
tags while release jobs hold write, identity, attestation, and Homebrew-token
credentials.

## Remediation direction

Pin every third-party action to a reviewed full commit SHA, update pins through
reviewed dependency automation, minimize job permissions, and isolate the
Homebrew token to the job that requires it.

## Implementation Notes

- Every third-party workflow action is pinned to a full commit SHA, with the
  source tag retained in a comment for maintenance review.
- Workflow-level write permissions were removed. Release attestation, release
  publication, Pages deployment, and Homebrew update jobs now receive only the
  permissions required for their individual operations.
- Checkout credentials are not persisted in worktrees; the Homebrew token is
  supplied only to the tap checkout and pull-request action in its dedicated
  job.
- Verification: Ruby YAML parsing succeeded for all three workflows; `git
  diff --check` passed. `actionlint` was unavailable in the environment.

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: `actionlint` remains unavailable locally; YAML parsing and direct
pin/permission inspection were used instead.

**Notes**: Standard substrate review with deep CI supply-chain/security lenses.
All third-party `uses:` references in CI, Pages, release, attestation, artifact,
and Homebrew jobs are full commit SHAs with maintenance comments. Workflow
permissions are least-scoped at job level, checkout credentials are disabled,
and the Homebrew token is confined to its checkout/PR job. Ruby YAML parsing and
`git diff --check` pass.
