---
id: epic-expanded-harness-support-trust-interactive-junie
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-trust-interactive
depends_on: [epic-expanded-harness-support-trust-interactive-contract-lock]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/junie-skills.md
  - .research/attestation/junie-mcp.md
  - .research/attestation/junie-extensions.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Junie Adapter

## Checkpoint

Implement Unit 2 from the parent feature: one distinct Junie adapter with exact
version-bounded skills/MCP observation and managed projection while preserving
interactive extension/cache state as read-only native evidence.

## Units

- Add `trust_interactive/junie.rs` with `JunieAdapter`,
  `JunieSkillProjection`, and the contract-locked effective-state decoder.
- Add `trust_interactive/junie_projection.rs` with the scoped, preserving
  `JunieMcpDocument` codec and `JunieManagedProjection` port.
- Export the adapter only after the contract-lock story proves a complete
  mutation/effective-observation profile.
- Consume the shared selected-source normalizer, exact-scope managed executor,
  profile gate, declared/effective normalization, and project-skill planner.

## Contract constraints

- Project standalone skills remain canonical under `.agents/skills` and receive
  only the shared relative per-skill link under `.junie/skills`; the adapter does
  not create links or duplicate trees.
- Edit only owned MCP server entries in documented scoped `mcp.json`; preserve
  unknown fields and unowned siblings, and block same-name conflicts/drift.
- Observe native extension declarations as `Declared`. Do not expose native
  extension mutation, write extension caches/state as an API, drive
  `/extensions`, or infer effective load from cache presence.
- MCP files are `Declared`; only the locked deterministic probe may emit
  `Effective`. Interactive-only evidence keeps this story blocked.
- Unknown profiles and narrowed capabilities perform no writes.

## Acceptance evidence

- Known/unknown detection and both-scope capability tests.
- Global complete skills and project canonical-plus-relative-link lifecycle,
  including repair, removal, complete siblings, and immediate-repeat no-op.
- Global/project MCP merge, precedence, unknown preservation, owned drift,
  conflict, removal, rollback, and repeat behavior.
- Declared/effective inactive, disabled, failed, and auth-required status with
  stable findings and no raw payload channels.
- Native extension declarations/caches remain byte-for-byte unchanged.

## Ordering

Consumes the locked native contract. The final acceptance story waits for this
and the Amp checkpoint; child verification advances directly to done without a
separate review pass.
