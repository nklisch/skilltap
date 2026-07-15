---
id: epic-expanded-harness-support-native-coexistence-qwen
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-native-coexistence
depends_on: [epic-expanded-harness-support-native-coexistence-contract]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/qwen-skills.md
  - .research/attestation/qwen-mcp.md
  - .research/attestation/qwen-extensions.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Qwen Code Adapter

## Checkpoint

Register `qwen` as a complete Qwen Code adapter with native extension source and
extension lifecycle, concrete Claude/Gemini conversion assessment, complete
standalone skills, scoped `settings.json` MCP projection, fresh-session load
verification, and exact compiled profile authority. Preserve Qwen-native
converted identity and enablement separately from managed component ownership.

## Files

- `crates/harnesses/src/adapters/qwen.rs` (new)
- `crates/harnesses/src/adapters/qwen_managed.rs` (new)
- `crates/harnesses/src/adapters/mod.rs`
- `crates/harnesses/src/registry.rs`
- `crates/harnesses/src/lib.rs`
- `crates/harnesses/tests/detection.rs`
- `crates/harnesses/tests/lifecycle_scope.rs`
- `crates/harnesses/tests/normalization.rs`
- `crates/test-support/src/harness_profile.rs`

## Adapter surface

```rust
pub struct QwenAdapter;
pub struct QwenLifecycle;
pub struct QwenSkillProjection;
pub struct QwenNativeDistribution;
pub struct QwenManagedProjection;

impl NativeDistributionPort for QwenNativeDistribution {
    fn assess(
        &self,
        context: &NativeDistributionContext<'_>,
    ) -> Result<Option<NativeDistributionAssessment>, NativeDistributionError>;
}

impl ManagedProjectionPort for QwenManagedProjection {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError>;
}
```

## Required behavior

- Refresh the current official `qwen` contract and validate a clean binary
  before adding exact profile constants. Pin version bytes, `extensions
  sources` and extension lifecycle argv, global/project (workspace alias)
  scope, structured observation, and fresh-session MCP probe.
- Native assessment recognizes only currently documented Claude, Gemini, npm,
  Git/local, and archive forms. Qwen owns conversion; skilltap records native
  converted identity rather than inventing a portable plugin identity.
- Parse the concrete source component graph before classifying conversion.
  Assess skills, MCP, agents, commands, context, hooks, and dependencies
  individually. A successful process exit is not proof of semantic equivalence.
- Global skills use `~/.qwen/skills`; project skills use `.qwen/skills` through
  the shared canonical-link contract. Extension-owned skills remain extension
  children and are not copied into standalone roots.
- Managed MCP edits only the `mcpServers` member in scoped
  `~/.qwen/settings.json` / `.qwen/settings.json`, preserving every unrelated
  member and server. Accept stdio, HTTP, or SSE only when command/url, args,
  environment/header references, auth, and transport remain faithful.
- After configuration changes, launch a new bounded Qwen process in the exact
  scope and observe load. A restart/session requirement is health evidence,
  never filesystem drift.
- Update/remove follows the pinned target representation. Native source,
  converted revision/enablement, managed projection manifest, and sibling
  target state remain independent.

## Acceptance evidence

- [x] Registry/help/config/dispatch expose `qwen` without expanding first-party
      bootstrap.
- [x] Exact profile grants only refreshed capabilities; neighboring/unknown
      versions remain observe-only and runtime probes only narrow.
- [x] Native source/extension install, conversion, update, enablement observation,
      uninstall, and immediate repeat pass at both scopes.
- [x] Conversion tests distinguish faithful, partial, blocked-required, malformed,
      and managed-strict-superset sources with exact component evidence.
- [x] Complete skills pass both scopes and shared project linking.
- [x] MCP codec tests preserve unknown settings, unrelated servers, references,
      transport semantics, conflicts, and owned removal.
- [x] Fresh-session verification, drift, pending recovery, partial native failure,
      and immediate repeat preserve exact target-local state.

## Implementation and verification

Implemented `QwenAdapter`, `QwenLifecycle`, `QwenNativeDistribution`,
`QwenManagedProjection`, the Qwen skill projection, human-only extension/source
and MCP decoders, and canonical registry exports. The implementation pins the
attested `0.19.10` profile, uses `workspace` for project lifecycle operations,
preserves native converted identity and enablement separately from managed
projection ownership, and restricts MCP edits to scoped `mcpServers` members.

Verification completed with:

- `cargo fmt --all -- --check`
- `cargo test --workspace --all-targets --no-fail-fast` — 699 passed
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`

The adjacent `0.19.11` detection fixture confirms observe-only authority and
zero writes. Qwen conversion, enablement, workspace scope, complete skills,
MCP transport, fresh-session, and coexistence tests use isolated fake bounded
processes and fixture roots.

## Ordering

Depends only on the coexistence contract. It may be implemented after Factory
to reuse the validated profile/source fixture shape; it does not depend on
Factory behavior.
