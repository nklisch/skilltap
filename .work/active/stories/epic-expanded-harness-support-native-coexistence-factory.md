---
id: epic-expanded-harness-support-native-coexistence-factory
kind: story
stage: implementing
tags: []
parent: epic-expanded-harness-support-native-coexistence
depends_on: [epic-expanded-harness-support-native-coexistence-contract]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/factory-skills.md
  - .research/attestation/factory-mcp.md
  - .research/attestation/factory-plugins.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Implement the Factory Droid Adapter

## Checkpoint

Register `droid` as a complete Factory Droid adapter with source-validated
native marketplace/plugin lifecycle, whole-directory skill roots, scoped MCP
configuration, native-managed distribution assessment, effective observation,
and exact compiled profile authority. Keep Factory-owned cache, identity, and
latest-marketplace-commit semantics separate from skilltap-managed projection.

## Files

- `crates/harnesses/src/adapters/factory.rs` (new)
- `crates/harnesses/src/adapters/factory_managed.rs` (new)
- `crates/harnesses/src/adapters/mod.rs`
- `crates/harnesses/src/registry.rs`
- `crates/harnesses/src/lib.rs`
- `crates/harnesses/tests/detection.rs`
- `crates/harnesses/tests/lifecycle_scope.rs`
- `crates/harnesses/tests/normalization.rs`
- `crates/test-support/src/harness_profile.rs`

## Adapter surface

```rust
pub struct FactoryAdapter;
pub struct FactoryLifecycle;
pub struct FactorySkillProjection;
pub struct FactoryNativeDistribution;
pub struct FactoryManagedProjection;

impl HarnessAdapter for FactoryAdapter {
    fn identity(&self) -> TargetIdentity; // droid / Factory Droid / Managed
    fn version_arguments(&self) -> Vec<OsString>;
    fn decode_version_with_limits(
        &self,
        stdout: &[u8],
        limits: JsonLimits,
    ) -> Result<NativeVersion, DetectionError>;
    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection;
    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError>;
    fn native_lifecycle(&self) -> Option<&dyn NativeLifecycleVector>;
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort>;
    fn native_distribution(&self) -> Option<&dyn NativeDistributionPort>;
    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort>;
    fn native_root(&self, paths: &PlatformPaths) -> Option<AbsolutePath>;
}
```

## Required behavior

- Refresh the current official Factory CLI contract and validate a clean binary
  before adding an exact profile. Pin version bytes, native argv, scoped list
  evidence, working directory, and postconditions in fixtures. Do not guess a
  version or grant authority from runtime probing.
- Native lifecycle uses the documented `droid plugin` marketplace/plugin
  commands with exact `user`/`project` scope through `NativeLifecyclePort`.
  Plugin caches are read-only observation evidence.
- Native distribution assessment recognizes only Factory-native and documented
  Claude-compatible forms after concrete component comparison. A requested pin
  cannot use Factory's unpinned latest-commit update semantics; select managed
  only when it can preserve the pin, otherwise block.
- Global standalone skills use `~/.factory/skills`; project skills use
  `.factory/skills` supplied to the shared canonical-link service. Preserve
  complete siblings and executable intent.
- Managed MCP merges owned entries into `~/.factory/mcp.json` or
  `.factory/mcp.json`, preserving unknown fields and unrelated servers. User
  definitions win collisions; a shadowed project entry is an effective-state
  finding, not drift and not permission to rewrite the user document.
- Auto-reload is proven through fresh native observation. Native plugin child
  skills/MCP stay native and are never duplicated as managed standalone
  resources.
- Required unsupported components block; optional omission remains explicit and
  foreground-acknowledged.

## Acceptance evidence

- Canonical registry/help/config/dispatch expose `droid`, while first-party
  skilltap bootstrap still yields only Codex and Claude.
- Exact known profile is mutation-authorized; neighboring/unknown versions are
  observe-only and probes only narrow.
- Both native scopes pass install/update/remove/list/post-observe and immediate
  repeat with separate qualified identity and latest-commit revision evidence.
- Both skill scopes preserve a complete directory; project scope proves the
  expected relative per-skill link and unmanaged sibling preservation.
- Both MCP scopes preserve unknown fields, references, and unrelated entries;
  user-over-project shadowing and malformed/unmanaged conflicts are distinct.
- Pinned source, optional/required unsupported components, drift, partial native
  failure, removal, and retry produce the expected target-local outcomes.

## Ordering

Depends only on the coexistence contract. It can become ready alongside Qwen
and Copilot, but remains a checkpoint inside one cohesive feature delivery.
