---
id: epic-expanded-harness-support-trust-interactive-acceptance
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-trust-interactive
depends_on: [epic-expanded-harness-support-trust-interactive-junie, epic-expanded-harness-support-trust-interactive-amp]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/junie-skills.md
  - .research/attestation/junie-mcp.md
  - .research/attestation/junie-extensions.md
  - .research/attestation/amp-manual.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Verify Junie and Amp Integration

## Checkpoint

Implement Unit 4 from the parent feature. Register the relaxed
contract-complete Junie and Amp declaration-managed adapters and prove the
full cross-unit behavior through shared adapter, managed-projection,
project-skill, and compiled CLI acceptance without invoking interactive state.

## Units

- Add final adapter exports and canonical registry entries in product order;
  retain Codex/Claude as the only first-party bootstrap targets.
- Add profile-carried Junie/Amp fake layouts and declaration-only responses; do
  not add target-id branches to test layout dispatch.
- Run `acceptance_matrix` and `managed_acceptance_matrix` with real assertions
  for each target before returning evidence labels, including pending
  declaration status.
- Add compiled CLI cases for registry exposure, both scopes, project-skill
  projection shapes, declared/effective divergence, Junie interactive/native
  preservation, and Amp trust/skill-local MCP without probes.

## Acceptance evidence

- Registry-derived help, enable/list/config/JSON/`--target all` includes both;
  bootstrap excludes both.
- Known exact profiles grant only locked capabilities; unknown versions/probe
  mismatches remain observe-only and perform no writes.
- Junie proves canonical tree plus relative `.junie/skills` link; Amp proves
  canonical `.agents/skills` no-link behavior.
- Both targets prove global/project source-only marketplace registration and
  complete skill+MCP install/update/remove, preservation, drift/conflict,
  acknowledgment, target-local state, pending recovery, rollback, and immediate
  repeat.
- Plain/JSON status distinguish declared, effective-unobserved,
  trust/auth/interactive-unverified, drift, and conflict from one typed outcome
  without raw payloads; Junie/Amp effective status stays unverified.
- Junie extension/cache state and Amp trust/auth state remain unmodified.
- Full workspace tests, all-feature Clippy with warnings denied, formatting, and
  `git diff --check` pass; no process invocation is used for effective health.

## Verification and ordering

Verified after both target checkpoints. The acceptance matrix covers both
scopes, exact/unknown profiles, declaration-managed status, preservation,
ownership/drift, repeatability/removal, and compiled version-only invocation
assertions. Amp's declared `mcp list --json` vector is asserted as metadata only;
no `doctor`, OAuth, or login invocation exists. Child stories close directly on
verification. This checkpoint makes the parent eligible for the requested
standard feature-level review; the parent remains at `stage: review` pending
that independent pass.
