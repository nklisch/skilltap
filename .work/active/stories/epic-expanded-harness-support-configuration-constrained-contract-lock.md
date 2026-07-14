---
id: epic-expanded-harness-support-configuration-constrained-contract-lock
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-configuration-constrained
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/kimi-mcp.md
  - .research/attestation/mistral-mcp.md
  - .research/attestation/kilo-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Lock Kimi, Vibe, and Kilo Native Contracts

## Checkpoint

Close the bounded evidence gap before any new adapter gains mutation authority.
Capture exact version command/output, global/project MCP documents and
precedence, and deterministic non-interactive effective-state probes for one
installed Kimi Code, Mistral Vibe, and Kilo Code release. Store only bounded,
non-secret fixtures and profile constants under the harnesses crate.

This is a contract-validation checkpoint, not a broad harness survey. Use the
existing source-direct URLs and isolated installations. Do not guess version
literals, parse unversioned human text, drive an interactive UI as an API, or
write production adapter registration in this story.

## Design element

Implement the feature's `VerifiedManagedTargetContract` and exact fixture set in:

- `crates/harnesses/src/adapters/configuration_constrained/contracts.rs`
- `crates/harnesses/tests/fixtures/configuration_constrained/{kimi,vibe,kilo}/`

Pin Kilo's precedence when both project `kilo.jsonc` and
`.kilo/kilo.jsonc` exist. Pin Vibe's exact named `[[mcp_servers]]` wire forms
and trust response. Pin Kimi's fresh-session probe and user/project override.

## Acceptance evidence

- Exact version bytes decode once and select only the matching compiled profile;
  malformed/extra/control/unknown output cannot authorize mutation.
- Exact scoped config fixtures cover supported transport/auth fields, unknown
  fields/comments, and precedence without credentials.
- Probe fixtures distinguish loaded, reload-required, trust-required,
  authentication-required, and failed state through bounded decoding.
- If any target has no deterministic non-interactive probe or reproducible
  write/reload boundary, record the blocker in the parent feature and leave
  that target unverified. Dependent work must not manufacture a workaround.

## Ordering

Foundation checkpoint. `projection-scope` is blocked until this evidence is
complete because its profile/probe interfaces must be proven by real contracts.
