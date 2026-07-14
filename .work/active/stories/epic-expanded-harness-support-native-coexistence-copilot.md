---
id: epic-expanded-harness-support-native-coexistence-copilot
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-native-coexistence
depends_on: [epic-expanded-harness-support-native-coexistence-contract]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/copilot-skills.md
  - .research/attestation/copilot-mcp.md
  - .research/attestation/copilot-plugins.md
  - .research/attestation/copilot-plugin-ref.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the GitHub Copilot CLI Adapter

## Checkpoint

Register `copilot` as a complete GitHub Copilot CLI adapter with native
marketplace/plugin lifecycle, canonical Agent Skills roots, scoped MCP
configuration and structured effective observation, policy/trust health, native-
managed source assessment, and exact compiled profile authority. Preserve
native plugin/declarative state, caches, and enterprise constraints separately
from skilltap-managed components.

## Files

- `crates/harnesses/src/adapters/copilot.rs` (new)
- `crates/harnesses/src/adapters/copilot_managed.rs` (new)
- `crates/harnesses/src/adapters/mod.rs`
- `crates/harnesses/src/registry.rs`
- `crates/harnesses/src/lib.rs`
- `crates/harnesses/tests/detection.rs`
- `crates/harnesses/tests/lifecycle_scope.rs`
- `crates/harnesses/tests/normalization.rs`
- `crates/test-support/src/harness_profile.rs`

## Adapter surface

```rust
pub struct CopilotAdapter;
pub struct CopilotLifecycle;
pub struct CopilotSkillProjection;
pub struct CopilotNativeDistribution;
pub struct CopilotManagedProjection;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CopilotEffectiveMcpObservation {
    pub declared: BTreeMap<NativeId, Fingerprint>,
    pub effective: BTreeMap<NativeId, Fingerprint>,
    pub policy: CopilotPolicyHealth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopilotPolicyHealth {
    Allowed,
    TrustRequired,
    EnterpriseBlocked,
    Unknown,
}
```

## Required behavior

- Refresh the current official Copilot CLI reference and validate a clean binary
  before adding exact profile constants. Pin version bytes, native marketplace/
  plugin argv and scope, repository working directory, structured plugin list,
  and `mcp list|get --json` schemas.
- Native assessment recognizes only documented Copilot `plugin.json` and Claude
  marketplace forms. Imperative lifecycle and declarative `enabledPlugins` are
  two native declared-state surfaces for one native identity, not duplicate
  desired resources. Installed plugin and marketplace caches are read-only.
- Global/project skill destinations are canonical `~/.agents/skills` and
  `<project>/.agents/skills`. The project link contract must return
  `NotRequired`; alternate `.github/skills` and `.claude/skills` are observed
  precedence/conflict surfaces, not extra managed copies.
- Managed MCP targets `~/.copilot/mcp-config.json` globally and
  `<project>/.mcp.json` for the project. Preserve `.github/mcp.json`, plugin MCP,
  unknown fields, unrelated servers, and credential references. Do not merge
  two repository files or replace a higher-precedence unmanaged declaration.
- Compare declared files with `copilot mcp list|get --json`. Repository trust or
  enterprise allowlist blocks produce stable attention findings; they do not
  become drift, mutation authority, or a reason to duplicate managed state.
- Native plugin child skills/MCP remain native. Managed fallback applies only
  when source assessment proves no faithful native distribution and follows
  normal required/optional component rules.
- Update/removal follows the pinned representation and exact target-local
  ownership; equal names/fingerprints never coalesce native and managed state.

## Acceptance evidence

- Registry/help/config/dispatch expose `copilot`, while first-party bootstrap
  remains Codex/Claude only.
- Exact profile grants only refreshed capabilities; neighboring/unknown versions
  are observe-only and probes only narrow.
- Native marketplace/plugin install, declared enablement, update, remove,
  structured list, post-observation, and immediate repeat pass at both scopes.
- Complete global/project skills preserve all siblings; project canonical root
  creates no link/copy and alternate roots remain intact.
- Managed MCP tests cover user/project files, precedence, alternate project
  declarations, unknown fields, references, drift, conflicts, and owned removal.
- Structured observation distinguishes declared/effective/policy states, with
  plain/JSON parity and no secret/native payload leakage.
- Mixed native/managed target state, partial failure, recovery, and immediate
  repeat preserve exact identities and ownership.

## Ordering

Depends only on the coexistence contract. It may follow Qwen for sequential
feature ownership but has no adapter-to-adapter dependency.
