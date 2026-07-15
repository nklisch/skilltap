---
id: epic-expanded-harness-support-trust-interactive-amp
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-trust-interactive
depends_on: [epic-expanded-harness-support-trust-interactive-contract-lock]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/amp-manual.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-15
---

# Implement the Amp Adapter

## Checkpoint

Implement Unit 3 from the parent feature: one distinct Amp declaration-managed
adapter with exact version-bounded managed skills/MCP, explicit
declared-versus-effective workspace-trust limits, and faithful preservation of
behavior-bearing skill-local MCP.

## Units

- Add `trust_interactive/amp.rs` with `AmpAdapter`, `AmpSkillProjection`, and
  the contract-locked bounded `AmpEffectiveStateProbe`.
- Add `trust_interactive/amp_projection.rs` with `AmpSettingsDocument`,
  `AmpMcpPlacement`, and `AmpManagedProjection`.
- Register/export Amp only after the contract-lock story pins the user settings
  path, precedence, profile, doctor output, and skill-local semantics.
- Consume the shared source normalizer, exact-scope projection/execution,
  target-local state, and project-skill service.

## Contract constraints

- Project `.agents/skills` is canonical and produces no redundant link/tree.
  Global managed skills use documented `~/.agents/skills`; other supported
  roots are observed for precedence/conflicts, not synchronized as copies.
- Edit only owned `amp.mcpServers` members in the locked user or nearest project
  settings document. Preserve unknown settings and unowned servers.
- Keep a skill-owned `mcp.json` inside its complete skill tree when relative
  paths or lazy activation are behavior-bearing; never duplicate it into scoped
  settings. Record exact skill and server fingerprints for drift/removal.
- `amp mcp doctor`, workspace trust, OAuth, and login are not invoked. The
  optional `mcp list --json` argument vector is finite declaration-only metadata;
  its bounded decoder never produces effective health, mutates trust, enters
  persistent policy, or widens compiled authority.
- Untrusted correct config stays declared/owned and attention-required, not
  drifted or repeatedly rewritten.

## Acceptance evidence

- Known/unknown profiles and both-scope managed capability routing.
- Amp project canonical no-link behavior and global single-root complete-skill
  lifecycle.
- User/workspace settings precedence, preserving merge/remove, same-name
  conflict, drift, target isolation, rollback, and repeat idempotency.
- Declared-only and effective-unobserved outcomes with stable secret-safe
  findings; no doctor/OAuth/login path is present.
- Skill-local relative-path/lazy MCP update and removal without a duplicate
  settings entry.
- Optional omissions require acknowledgment; required unsupported behavior
  blocks under `--yes`.

## Ordering

Consumes the locked native contract. Verified on both scopes with exact
identity, source/config preservation, precedence/conflict, ownership/drift,
removal, repeatability, unknown version, bounded declaration decoding, and
explicit version-only native invocation assertions. The final acceptance story
waits for this and the Junie checkpoint; child verification advances directly
to done without a separate review pass.
