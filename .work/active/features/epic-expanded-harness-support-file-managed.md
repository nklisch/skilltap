---
id: epic-expanded-harness-support-file-managed
kind: feature
stage: done
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-project-skill-links, epic-expanded-harness-support-declaration-managed]
release_binding: 3.1.0
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-15
---

# File-Managed Adapters for Gemini, OpenCode, and Kiro

## Brief

Deliver complete adapters for Gemini CLI, OpenCode, and Kiro CLI using their
documented global and project skill roots, MCP configuration, effective-state
observation, and reload or status mechanisms. Each adapter exposes its own
verified version profile and target semantics while consuming the shared
managed projection lifecycle for complete skills, MCP entries, ownership,
drift, update, and removal.

These targets form the direct file-managed group because their supported write
and observation boundaries are explicit and do not require a native marketplace
to be useful. The feature includes each target's isolated native validation,
agent-facing help/status exposure, and shared acceptance-contract evidence. It
does not broaden first-party plugin bootstrap or treat project trust as proof of
effective load.

## Review result

The required standard independent pass approved the completed family with no
material findings. Focused Kiro, declaration-authority, daemon, compiled CLI,
and Gemini/OpenCode parity tests passed. Kiro is registered only through the
new exact-version declaration-managed path; its original attestation correctly
retains the unresolved native effective-observation result. A future research
refresh may add a forward pointer, and the generic compiled-test shell quoting
helper may move when another fixture needs it; neither affects behavior.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: parallel concrete-adapter feature after the registry and
  managed fallback foundations.

## Simplification opportunity

- Reuse one managed skill/MCP transaction and one acceptance harness; delete
  adapter-local copies of ownership, rollback, and idempotency logic.

## Foundation references

- `docs/VISION.md` — Native First, Deep Support Over Broad Claims.
- `docs/ARCH.md` — Harness Adapter Contract, Observation, Plugin Resolution.
- `docs/HARNESS-CONTRACTS.md` — Expanded Target Set, Adding Another Harness.

## Grounding summary

The completed foundations are usable but expose two narrow gaps that this first
file-managed family must close rather than work around:

- `TargetRegistry`, `HarnessAdapter`, `SkillProjectionPort`, exact-profile
  selection, registry-derived CLI validation/help, and `FakeHarnessProfile` are
  implemented in `crates/harnesses/src/registry.rs` and test support. The
  canonical registry currently contains only Codex and Claude.
- Project standalone skills now have one canonical
  `<project>/.agents/skills/<name>` tree. `SkillProjectionPort::destination`
  decides whether a target consumes it directly or receives a relative
  per-skill link. Concrete adapters must supply only their native root and
  compatibility evidence; they must not implement link ownership or repair.
- `ManagedProjectionPort::plan` already returns target-native tree/file writes,
  the exact projection manifest, and current/desired fingerprints. The shared
  CLI path owns source checkout, ownership, drift, acknowledgment, pending
  recovery, rollback, target-local state, and immediate-repeat behavior.
- That shared path is currently named and gated as project-only:
  `ManagedProjectionContext` requires `project`, `HarnessAdapter` exposes
  `managed_project_lifecycle()`, and `plan_managed_project_lifecycle` runs only
  for `Scope::Project`. A target with no native package lifecycle therefore has
  no global plugin materialization route yet.
- `HarnessAdapter::observe` can snapshot documented files, but the adapter
  contract has no bounded native status-probe port. Gemini and OpenCode require
  CLI MCP status to distinguish declared files from effective state; Kiro's
  documented hot reload is verified by MCP status. Raw human output cannot be
  allowed into findings or treated as stable across unknown versions.
- `TargetIdentity` assumes the harness id is also the default executable name.
  Kiro's documented executable is `kiro-cli`, so the registry must own an exact
  `default_binary` instead of adding a CLI special case.
- Existing managed source extraction in
  `crates/harnesses/src/adapters/codex_managed.rs` already safely resolves the
  selected local marketplace entry, snapshots a complete plugin tree, discovers
  complete skills and MCP declarations, classifies unsupported components, and
  rejects plugin-root-relative MCP commands. The file-managed adapters need this
  source-side behavior but not Codex's destination TOML codec.
- `FakeHarnessProfile::layout` still matches on `"codex"` and `"claude"`.
  Adding three more branches would recreate the target list the registry feature
  removed; the layout must become profile data.

The exact researched native boundaries are:

| Target | Complete skills | MCP declaration | Effective/reload evidence |
|---|---|---|---|
| Gemini CLI | global `~/.gemini/skills` or `~/.agents/skills`; workspace `.gemini/skills` or `.agents/skills` | global `~/.gemini/settings.json`; project `.gemini/settings.json`; `mcpServers` | `gemini mcp list`; `/mcp reload` is interactive; project config is ineffective while the workspace is untrusted |
| OpenCode | own, Claude-compatible, or `.agents/skills` global/project roots | global `${XDG_CONFIG_HOME:-~/.config}/opencode/opencode.json`; project `opencode.json`; `mcp` | `opencode mcp list` and `opencode mcp debug`; OAuth tokens and Bun cache are separate |
| Kiro CLI | `${KIRO_HOME:-~/.kiro}/skills`; project `.kiro/skills`, with project precedence | `${KIRO_HOME:-~/.kiro}/settings/mcp.json`; project `.kiro/settings/mcp.json`; `mcpServers` | file hot reload and `kiro-cli mcp list`; `/mcp` is interactive |

No source-direct research artifact records exact installed version strings or
exact `--version` output bytes for these three binaries. The design therefore
does not invent version numbers. The first checkpoint performs the brief's
isolated native validation and pins the observed exact version/output contract
before any adapter enters `TargetRegistry::canonical()`. A target whose exact
profile cannot be validated remains unregistered and cannot mutate.

The mapping used direct reading only. The caller prohibited agents and peers;
child stories are durable contract checkpoints for one cohesive Sol-high
feature worker, not parallel worker assignments.

## Design decisions

- **File-managed lifecycle applies at both scopes.** Generalize the existing
  managed projection path from `project: &AbsolutePath` to an exact `Scope`.
  Codex continues to opt in only for project scope; Gemini, OpenCode, and Kiro
  opt in for global and project. Native lifecycle remains preferred whenever an
  adapter exposes it, so this extension does not redirect Codex or Claude global
  operations.
- **Exact version authority is validation-produced, not guessed.** The contract
  checkpoint captures each installed binary's exact `--version` bytes, pins one
  exact `NativeVersion` and profile id, and fixture-tests near versions as
  observe-only. Documentation date is not a substitute for a native version.
- **Registry owns the default executable.** Add
  `TargetIdentity::default_binary: &'static str`; Codex=`codex`,
  Claude=`claude`, Gemini=`gemini`, OpenCode=`opencode`, Kiro=`kiro-cli`.
  Configuration creation and detection derive from it. Existing explicit
  `--binary` overrides remain authoritative.
- **Three adapters, one source-side projection helper.** `GeminiAdapter`,
  `OpenCodeAdapter`, and `KiroAdapter` remain distinct and own paths, version
  decoding, profile ids, MCP codecs, probe decoding, reload semantics, and
  compatibility. Shared helper code only reads the already-supported selected
  marketplace/plugin source into complete skill trees, portable MCP server
  values, and unsupported-component evidence.
- **No native package lifecycle claims.** Gemini's extension lifecycle is not
  used because this feature manages portable plugin components and Gemini has no
  registered custom-marketplace contract. OpenCode's one-way `plugin` command
  is not a complete update/remove/list lifecycle and its Bun cache is read-only.
  Kiro Powers are an IDE package contract and remain out of scope. All three
  return `native_lifecycle() == None` and use managed projection.
- **Marketplace registration may be control-plane-only.** For these adapters,
  marketplace add/update/remove records the explicit source and revision in
  inventory/target-local state but writes no fake native catalog. The shared
  orchestrator accepts an empty adapter plan for `ResourceKind::Marketplace`
  through a source-registration operation; empty plugin plans remain invalid.
- **Project standalone skills use the completed link contract.** Gemini and
  OpenCode choose project `.agents/skills`, so their projections are canonical
  no-ops. Kiro chooses `.kiro/skills`, so the existing project-skill planner
  derives and owns relative links. Adapter code never creates a symlink.
  Plugin-owned skill components remain managed projection trees in each
  target's documented load root because they are components of one plugin
  lifecycle, not separately adopted standalone resources.
- **Global skill roots are target-exact.** Gemini and OpenCode use the portable
  global `~/.agents/skills` root documented by their loaders. Kiro uses
  `${KIRO_HOME:-~/.kiro}/skills`. This avoids redundant Gemini/OpenCode copies
  while retaining Kiro's actual load boundary.
- **JSON merges preserve unrelated fields and servers.** Each adapter parses a
  bounded strict JSON object, edits only its native MCP container and only
  skilltap-owned server names, and serializes the preserved object. A malformed
  document, duplicate key, non-object container, or same-name unowned server is
  a typed conflict. OpenCode's source-to-native MCP transformation stays in its
  codec; Gemini and Kiro do not share its schema by coincidence.
- **Effective state is a separate port.** Add a bounded
  `EffectiveStateProbePort` that supplies direct argv, scope working directory,
  an exact-version decoder, and reload semantics. The CLI runs it with the
  already resolved executable and `NativeProcessRunner`; adapters receive only
  bounded stdout and return typed server health/trust evidence. Raw output,
  argv, settings bytes, and parser messages never enter findings.
- **Interactive reload is never automated.** Gemini `/mcp reload` and Kiro
  `/mcp` are interactive commands. Gemini uses `gemini mcp list` for fresh
  status and emits an actionable reload/session next action when needed. Kiro's
  file watcher plus `kiro-cli mcp list` is the non-interactive verification.
  OpenCode re-reads layered configuration and verifies with `opencode mcp list`.
- **Gemini trust is health, not drift.** A project settings file may be declared
  correctly while ignored in an untrusted workspace. The adapter reports
  `attention_required` and load verification remains pending; it never rewrites
  trust or claims the resource effective.
- **OpenCode secret/cache boundaries remain outside state.** OAuth tokens,
  literal secret values, and `~/.cache/opencode` are neither read as desired
  state nor written. Environment/header references remain references.
- **Kiro home is a platform path.** Extend `EnvironmentVariable` and
  `PlatformPaths` with optional `KIRO_HOME`, including the bounded child roots.
  Do not read ambient `KIRO_HOME` directly inside the adapter.
- **Review and execution posture.** One Sol-high worker owns the cohesive
  feature and its checkpoints. Effective review weight is standard: one
  independent feature-level pass after child verification, then receiver
  adjudication/fixes/verification without a second pass.

## Architectural choice

**Chosen — distinct adapters over a narrow file-managed toolkit.** Extend the
existing registry and managed-projection contracts only where the first
non-native targets prove they are incomplete: exact default binary metadata,
concrete scope, control-plane-only source registration, and bounded effective
status probing. A private `file_managed` module normalizes the selected source
plugin and performs common ownership-aware tree/fingerprint work; each adapter
owns a separate MCP codec and probe decoder.

**Rejected — one descriptor-driven `FileManagedAdapter`.** A table of roots and
JSON keys looks short but hides meaningful differences: Gemini trust and
interactive reload, OpenCode's `mcp` schema and separate OAuth/cache state, and
Kiro's `KIRO_HOME`, native skill root, hot reload, and Power exclusion. It would
be a universal plugin format by another name.

**Rejected — copy Codex managed adapter three times.** This duplicates source
reading, ownership checks, omission rules, fingerprints, rollback, and tests,
and risks carrying Codex TOML or catalog semantics into unrelated targets.

**Rejected — call native CLIs from `HarnessAdapter::observe`.** That method has
no resolved executable identity, bounded runner, process limits, or explicit
working directory. An explicit probe port keeps process execution at the
existing boundary and lets unknown profiles fail closed.

## Implementation Units

### Unit 1: Scope-aware managed and effective-state contracts

**Files**:

- `crates/core/src/runtime/environment.rs` and `crates/core/src/runtime/paths.rs`
  — add validated `KIRO_HOME` resolution.
- `crates/core/src/managed_projection.rs` — retain write/evidence currency;
  document scope-neutral use.
- `crates/harnesses/src/registry.rs` — default binary, scoped managed support,
  and effective-state probe accessor.
- `crates/harnesses/src/effective_state.rs` (new) — bounded probe contract.
- `crates/harnesses/src/managed_projection.rs` — exact scope replaces project.
- `crates/harnesses/src/adapters/file_managed.rs` (new) — shared selected-source
  reader, complete-skill planner, omission/fingerprint helpers, and bounded JSON
  object merge support.
- `crates/cli/src/application.rs` and
  `crates/cli/src/application/lifecycle.rs` — scope-neutral planning/execution,
  source-only marketplace operations, and exact-profile gate.
- `crates/cli/src/application/status.rs` — run and normalize bounded effective
  probes after detection.
- `crates/cli/src/application/execution.rs` — rename project-only managed entry
  types to scope-neutral names without changing revalidation/rollback behavior.
- `crates/test-support/src/harness_profile.rs` and
  `crates/test-support/src/managed_acceptance.rs` — profile-carried layouts and
  both-scope managed acceptance.

**Story**: `epic-expanded-harness-support-file-managed-contracts`

```rust
pub struct TargetIdentity {
    pub id: HarnessId,
    pub display_name: &'static str,
    pub default_binary: &'static str,
    pub distribution_surface: DistributionSurface,
}

pub trait HarnessAdapter: Sync {
    // existing required methods
    fn supports_managed_projection(&self, scope: CapabilityScope) -> bool {
        false
    }
    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort> {
        None
    }
    fn effective_state_probe(&self) -> Option<&dyn EffectiveStateProbePort> {
        None
    }
}

pub struct ManagedProjectionContext<'a> {
    pub target: &'a HarnessId,
    pub scope: &'a Scope,
    pub paths: &'a PlatformPaths,
    pub resource_key: &'a ResourceKey,
    pub resource_kind: ResourceKind,
    pub request: &'a NativeLifecycleRequest,
    pub kind: ManagedLifecycleKind,
    pub input: ManagedProjectionInput<'a>,
    pub prior: &'a [ManagedProjection],
    pub acknowledged: bool,
    pub filesystem: &'a dyn ConfinedFileSystem,
    pub json_limits: JsonLimits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReloadSemantics {
    HotReload,
    StatusRefresh,
    InteractiveRequired { next_action: &'static str },
}

pub struct EffectiveProbeSpec {
    pub arguments: Vec<OsString>,
    pub working_directory: Option<AbsolutePath>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveMcpStatus {
    pub servers: BTreeMap<NativeId, EffectiveServerHealth>,
    pub project_trust: Option<ProjectTrustHealth>,
}

pub trait EffectiveStateProbePort: Sync {
    fn mcp_status_spec(&self, scope: &Scope) -> EffectiveProbeSpec;
    fn decode_mcp_status(
        &self,
        stdout: &[u8],
        limits: JsonLimits,
    ) -> Result<EffectiveMcpStatus, EffectiveProbeError>;
    fn reload_semantics(&self) -> ReloadSemantics;
}
```

**Implementation notes**:

- Replace `managed_project_lifecycle() -> bool` with
  `supports_managed_projection(CapabilityScope)`. Codex returns project only;
  new adapters return both scopes. Claude remains false.
- `plan_managed_project_lifecycle` becomes `plan_managed_lifecycle` and derives
  the exact scope from `resource.key()`. No adapter may return a write outside
  its documented global root or selected project root; the existing confined
  execution port revalidates every returned root/destination under the lock.
- Before any managed plan, detection must select the exact verified profile and
  the scope's semantic plugin/marketplace capability must be `Supported`.
  Unknown versions and narrowed profiles remain observe-only.
- `ResourceKind::Marketplace` may return an empty plan only for adapters that
  advertise source-only registration. Add a dedicated operation reason and
  execution entry for this state-only change. `PluginInstall`/`PluginUpdate`
  with no tree/file writes remains an error.
- `file_managed` chooses the existing compatible marketplace document, resolves
  the exact selected plugin, accepts either the validated Codex or Claude plugin
  manifest reader, snapshots one no-symlink complete tree, and returns named
  complete skill subtrees plus MCP values. Unsupported required components use
  `RequiredUnsupported`; optional components become `Omitted` only after
  acknowledgment. It does not know destination paths or native MCP schema.
- Probe decoders are exact-profile code. A parse mismatch is
  `effective_state_unverified`, never a healthy empty server list. Normalized
  findings use registered codes and typed bounded fields only.
- Make acceptance layout data (`global skill root`, `project skill root`, MCP
  documents, reload fixture) part of `FakeHarnessProfile`; delete the id match
  in `layout()`.

**Acceptance criteria**:

- [ ] Codex project managed tests remain unchanged in outcome; Codex global and
      Claude scopes do not change route.
- [ ] A fake managed-only adapter can register a source and install/update/remove
      one complete skill+MCP plugin globally and in a project, with immediate
      repeats producing no operation.
- [ ] An unknown version cannot enter managed apply even when the adapter has a
      managed projection port.
- [ ] Empty source-registration plans are accepted only for marketplace actions;
      empty plugin plans fail.
- [ ] Exact version/output bytes for Gemini, OpenCode, and Kiro are captured in
      isolated roots and pinned before canonical registration; adjacent/unknown
      versions are observe-only.
- [ ] Probe invocation uses the resolved executable, direct argv, bounded output,
      explicit environment/working directory, and secret-safe typed failures.
- [ ] Existing config defaults remain byte-compatible for Codex/Claude, while a
      newly enabled Kiro target defaults to `kiro-cli`.

### Unit 2: Gemini CLI adapter

**Files**: `crates/harnesses/src/adapters/gemini.rs` and
`crates/harnesses/src/adapters/gemini_managed.rs` (new), plus adapter module
exports and the canonical registry entry.

**Story**: `epic-expanded-harness-support-file-managed-gemini`

```rust
pub struct GeminiAdapter;
pub struct GeminiSkillProjection;
pub struct GeminiManagedProjection;
pub struct GeminiEffectiveStateProbe;

impl HarnessAdapter for GeminiAdapter {
    fn identity(&self) -> TargetIdentity;
    fn version_arguments(&self) -> Vec<OsString>;
    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, DetectionError>;
    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection;
    fn observe(&self, paths: &PlatformPaths, scope: &Scope, limits: ExternalTreeLimits)
        -> Result<AdapterObservationPaths, ObservationPathError>;
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort>;
    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort>;
    fn supports_managed_projection(&self, scope: CapabilityScope) -> bool;
    fn effective_state_probe(&self) -> Option<&dyn EffectiveStateProbePort>;
}
```

**Implementation notes**:

- Identity is `gemini` / `Gemini CLI`, default binary `gemini`, managed
  distribution. Register only after Unit 1 pins the exact profile.
- Skill destination is `~/.agents/skills` globally and
  `<project>/.agents/skills` in projects. Also observe `.gemini/skills` as an
  unmanaged native precedence surface; never delete or merge it implicitly.
- MCP codec edits only `mcpServers` in `~/.gemini/settings.json` or
  `<project>/.gemini/settings.json`, preserving every unrelated setting and
  server. Preserve documented stdio/HTTP/SSE semantics only when source fields
  map exactly; reject or acknowledge unsupported differences.
- Probe argv is exactly `gemini mcp list`, with project root as cwd for project
  scope. Decoder and trust markers are pinned to the validated version. The
  interactive `/mcp reload` is surfaced as a next action, not invoked.
- Project declared resources remain ineffective while trust evidence is absent
  or untrusted. This blocks fresh-load verification but does not classify file
  contents as drift.
- Do not expose Gemini extension install/update/remove or marketplace
  capabilities in this feature.

**Acceptance criteria**:

- [ ] Both scopes observe complete skill directories and merged MCP settings;
      global/project same-name precedence matches the documented Gemini order.
- [ ] Correct project standalone skills require no redundant link because the
      native destination equals the canonical root.
- [ ] Unrelated settings, native `.gemini/skills`, and unmanaged MCP servers are
      preserved byte-semantically across install/update/remove.
- [ ] Trusted status verifies effective MCP; untrusted/unknown status produces
      attention required and no false ownership success.
- [ ] Optional unsupported components require `--yes`; required unsupported
      components block even with acknowledgment.
- [ ] No Gemini extension directory/cache is written.

### Unit 3: OpenCode adapter

**Files**: `crates/harnesses/src/adapters/opencode.rs` and
`crates/harnesses/src/adapters/opencode_managed.rs` (new), plus module exports
and the canonical registry entry.

**Story**: `epic-expanded-harness-support-file-managed-opencode`

```rust
pub struct OpenCodeAdapter;
pub struct OpenCodeSkillProjection;
pub struct OpenCodeManagedProjection;
pub struct OpenCodeEffectiveStateProbe;

pub(crate) struct OpenCodeMcpCodec;
impl OpenCodeMcpCodec {
    fn encode_server(source: &PortableMcpServer)
        -> Result<serde_json::Value, ManagedProjectionError>;
}
```

**Implementation notes**:

- Identity is `opencode` / `OpenCode`, default binary `opencode`, managed
  distribution. Register only with the exact Unit 1 profile.
- Skill destination is the documented portable `.agents/skills` root in both
  scopes. Observe OpenCode's own and Claude-compatible skill roots as native
  precedence surfaces without adopting or rewriting them.
- Global MCP destination is
  `${XDG_CONFIG_HOME:-~/.config}/opencode/opencode.json`; project destination is
  `<project>/opencode.json`. Edit only the `mcp` object.
- `OpenCodeMcpCodec` performs an explicit source-to-native conversion for local
  and remote definitions, including native `type`, command vector, environment,
  URL, headers, enabled state, and documented tool filtering. It rejects
  ambiguous transport, literal secret material, OAuth state, or fields without
  an attested equivalent. It never treats a copied `mcpServers` value as native
  OpenCode schema.
- Probe argv is `opencode mcp list`; `opencode mcp debug` is emitted only as a
  diagnostic next action after a failed server, not run during ordinary status.
- Never invoke `opencode plugin`, edit npm plugin config, or write
  `~/.cache/opencode`/Bun dependencies.

**Acceptance criteria**:

- [ ] Local and remote MCP fixtures map to exact OpenCode native JSON and are
      visible through the version-pinned status decoder in both scopes.
- [ ] Project values override same-named global values without deleting either
      declaration; removal restores the effective global server.
- [ ] Unknown config keys, unrelated plugins/settings, and unmanaged MCP entries
      survive; same-name unowned entries conflict rather than overwrite.
- [ ] OAuth/token/cache material never enters inventory, state, findings, or
      writes.
- [ ] Complete skills use the canonical project tree without a redundant link.
- [ ] Immediate install/update/remove repeats are no-ops.

### Unit 4: Kiro CLI adapter

**Files**: `crates/harnesses/src/adapters/kiro.rs` and
`crates/harnesses/src/adapters/kiro_managed.rs` (new), plus module exports and
the canonical registry entry.

**Story**: `epic-expanded-harness-support-file-managed-kiro`

```rust
pub struct KiroAdapter;
pub struct KiroSkillProjection;
pub struct KiroManagedProjection;
pub struct KiroEffectiveStateProbe;
```

**Implementation notes**:

- Identity is `kiro` / `Kiro CLI`, default binary `kiro-cli`, managed
  distribution. Resolve `${KIRO_HOME:-~/.kiro}` only through `PlatformPaths`.
- Standalone skills use `<kiro-home>/skills` globally and
  `<project>/.kiro/skills` in projects. The latter causes the completed project
  skill lifecycle to create a relative per-skill link to canonical
  `.agents/skills`; the adapter adds no link code.
- Managed plugin skill components write complete owned trees to the same Kiro
  native skill roots under the shared managed transaction. Their plugin
  ownership/state remains separate from standalone canonical skill resources.
- MCP destinations are `<kiro-home>/settings/mcp.json` and
  `<project>/.kiro/settings/mcp.json`. Edit only `mcpServers`; preserve
  unrelated document fields, disabled state, tool filters, and unowned servers.
- Probe argv is `kiro-cli mcp list`, with project cwd at project scope. File hot
  reload plus fresh list output verifies the post-write state. `/mcp` remains an
  interactive diagnostic only.
- Kiro Powers (`POWER.md`, hooks, steering, IDE installation) are not translated
  or advertised. If a source plugin requires those semantics, classify it
  partial or blocked by requiredness.

**Acceptance criteria**:

- [ ] `KIRO_HOME` override and default home produce exact global roots without
      affecting canonical `~/AGENTS.md` or other harness homes.
- [ ] A project standalone skill creates the shared planner's correct relative
      link; drift/repair/removal follow the existing no-follow ownership rules.
- [ ] Managed plugin skill/MCP install, hot reload observation, update, removal,
      rollback, and repeat idempotency pass in both scopes.
- [ ] Workspace MCP precedence and disabled/tool-filter semantics are preserved.
- [ ] Powers and IDE caches are untouched and unsupported required Power
      components block.

### Unit 5: Integrated registry, lifecycle, status, and acceptance evidence

**Files**:

- `crates/harnesses/src/adapters/mod.rs`, `crates/harnesses/src/lib.rs`, and
  `crates/harnesses/src/registry.rs` — final exports and canonical order.
- `crates/harnesses/tests/detection.rs` and adapter contract tests — exact
  detection/profile/path/probe behavior.
- `crates/test-support/src/harness_profile.rs` and
  `crates/test-support/src/managed_acceptance.rs` — concrete profiles.
- `crates/cli/src/application/tests.rs` and
  `crates/cli/tests/compiled_binary.rs` — full lifecycle, output, and repeat
  acceptance.

**Story**: `epic-expanded-harness-support-file-managed-acceptance`

**Implementation notes**:

- Canonical registry order becomes `codex`, `claude`, `gemini`, `opencode`,
  `kiro`. Help, validation, enabled resolution, `--target all`, status labels,
  and config policy derive from that one order.
- Add `FakeHarnessProfile::{gemini, opencode, kiro}` and matching managed
  projection profiles. The shared fixture layout has no target-id match.
- Run both the adapter acceptance matrix and managed projection matrix for each
  target. The production-aware matrix must assert behavior before returning
  evidence labels; labels alone are not proof.
- Use isolated HOME, XDG, KIRO_HOME, projects, sources, and fake binaries. Never
  invoke real harness binaries in ordinary tests.

**Acceptance criteria**:

- [ ] `harness list`, `harness enable`, help, JSON, and `--target all` expose all
      five canonical targets while first-party bootstrap still exposes only
      Codex and Claude.
- [ ] Each new exact profile is mutable in both scopes; unknown or malformed
      version output remains reachable/observe-only or fails typed detection as
      appropriate, never mutating.
- [ ] Each adapter passes detection, both scopes, complete skill siblings and
      executable intent, MCP merge/precedence/secrets, effective status/reload,
      drift, partial/required compatibility, owned removal, target-local state,
      pending recovery, rollback, and immediate-repeat acceptance.
- [ ] Kiro project link behavior and Gemini/OpenCode canonical no-link behavior
      are covered through compiled CLI tests.
- [ ] Plain and JSON status derive from one outcome and distinguish declared,
      effective, untrusted/unverified, drifted, and conflict states.
- [ ] Native extension/plugin/Powers/cache paths remain byte-for-byte untouched.
- [ ] `cargo test --workspace --all-targets`, all-feature Clippy with warnings
      denied, formatting, and `git diff --check` pass.

## Implementation Order

1. `epic-expanded-harness-support-file-managed-contracts` — Unit 1,
   `depends_on: []`.
2. `epic-expanded-harness-support-file-managed-gemini` — Unit 2,
   `depends_on: [epic-expanded-harness-support-file-managed-contracts]`.
3. `epic-expanded-harness-support-file-managed-opencode` — Unit 3,
   `depends_on: [epic-expanded-harness-support-file-managed-contracts]`.
4. `epic-expanded-harness-support-file-managed-kiro` — Unit 4,
   `depends_on: [epic-expanded-harness-support-file-managed-contracts]`.
5. `epic-expanded-harness-support-file-managed-acceptance` — Unit 5,
   `depends_on: [epic-expanded-harness-support-file-managed-gemini,
   epic-expanded-harness-support-file-managed-opencode,
   epic-expanded-harness-support-file-managed-kiro]`.

The three adapter checkpoints are dependency-parallel because their only real
prerequisite is the shared contract. The normal execution remains one Sol-high
feature owner, implementing Gemini first, then OpenCode, then Kiro to preserve
cohesive context. The final acceptance checkpoint waits for all three. Cycle
checks with `.work/bin/work-view --blocking <story-id>` returned no existing
edges for every story before these dependencies were written.

## Implementation amendment: Kiro declaration-managed completion

The original design's Kiro unit was deliberately provisional because the
attestation proved the documented declaration files but did not prove a safe
non-interactive effective probe. The relaxed completion consumes that evidence
without weakening the declaration boundary:

- `KiroAdapter` is now exported and registered at exact profile `kiro-2-12-2`
  with registry-owned default executable `kiro-cli`. Existing target ordering is
  preserved; Kiro follows OpenCode without reshaping unrelated families.
- Kiro's exact global and project capabilities include only the documented
  observe, complete-skill, and MCP declaration surfaces. Native lifecycle,
  Powers, authentication, trust, and effective-load capabilities remain absent.
  The profile is `Unverified` for managed projection and component skill/MCP
  declaration evidence in both scopes; adjacent and unknown versions remain
  observe-only.
- The adapter explicitly supplies a
  `ManagedDeclarationContract` covering exactly `ManagedDocument` and
  `CompleteSkillTree`. Kiro writes use the existing confined managed
  transaction, ownership/fingerprint revalidation, rollback, and repeat
  no-op behavior. Foreground `--yes` acknowledges the effective-unverified
  consequence; the daemon never acknowledges or constructs the declaration
  write.
- The provisional Kiro effective probe was removed. No `kiro-cli mcp list`,
  login, trust, interactive `/mcp`, cache, or Power path is invoked or written;
  status remains attention-required with declared ownership separate from
  effective-unverified state.
- The existing foreground managed route now admits exact `Unverified` profiles
  to plan their partial operation. The shared executor still blocks without
  the exact foreground acknowledgment and rejects unsupported/conflicted/
  drifted/invalid operations. Native lifecycle routes remain Supported-only.
- Compiled isolated acceptance covers both scopes, exact JSON declarations,
  project links, no-ack blocking, effective-unverified status, daemon target
  no-write behavior, idempotent repeats, unknown/adjacent versions, and
  absence of login/native MCP/cache/Power activity. Gemini/OpenCode fixture and
  managed acceptance profiles run as regressions through the shared matrices.

The Kiro and integrated acceptance stories are done. This feature is now at
review; the parent `epic-expanded-harness-support` is intentionally not
reviewed by this implementation checkpoint.

## Checkpoint commits

- `0245ac42` — `implement: epic-expanded-harness-support-file-managed-kiro`
- `7896f73a` — `implement: epic-expanded-harness-support-file-managed-acceptance`
- The parent feature checkpoint follows this amendment as
  `implement: epic-expanded-harness-support-file-managed`.

## Simplification

- Replace `managed_project_lifecycle`/`plan_managed_project_lifecycle` and their
  project-only entry names with one exact-scope managed path; do not add a
  second global orchestrator.
- Replace CLI assumptions that target id equals binary name with registry
  metadata; do not add a Kiro match.
- Replace `FakeHarnessProfile::layout` target-id branching with profile data.
- Extract source-side plugin reading, complete-skill slicing, omission handling,
  and aggregate fingerprinting from the Codex destination codec; retain Codex's
  catalog/TOML details in Codex modules.
- Reuse project-skill canonical/link lifecycle for standalone skills; adapters
  supply roots and compatibility only.
- Reuse shared managed ownership, target-local state, rollback, and acceptance;
  adapters return target-native plans and typed probe evidence only.
- Do not add native marketplace manifests, extension caches, plugin caches,
  Powers translation, OAuth storage, or a generic descriptor adapter.

No cleanup child is warranted: each removal is coupled to the contract or test
replacement that proves it.

## Testing

- **Contract tests:** exact default binaries, exact version decode/profile
  selection, unknown-version no-mutation, scope routing, state-only marketplace
  registration, bounded probe object safety, and profile-data fixture layouts.
- **Codec tests:** preserve unknown JSON fields and unowned sibling servers;
  reject malformed/duplicate documents, unowned same-name entries, ambiguous
  transports, literal secrets, and unsupported required fields. Pin each native
  container/schema independently.
- **Project-link tests:** Gemini/OpenCode canonical no-op and Kiro relative link,
  including nested project roots and no-follow repair/removal inherited from the
  completed contract.
- **Managed acceptance:** both scopes, complete skill trees, MCP evidence,
  acknowledgment, ownership, drift, pending recovery, fresh effective load,
  target isolation, rollback, removal, and immediate repeat for all three.
- **Compiled CLI tests:** registry-derived help/enable/list/all, first-party
  bootstrap exclusion, plain/JSON status parity, trust/unverified attention,
  and no writes to native caches.
- **Test economy:** do not snapshot full help or every JSON byte ordering; pin
  stable ids, paths, native schema values, findings, operation surfaces, and
  externally visible results. Existing Codex/Claude acceptance remains the
  regression baseline.

## Risks

- **Exact profile evidence is not present in the research artifact.** This is
  the riskiest assumption and cannot be repaired by guessing. Unit 1 validates
  actual isolated binaries and records exact bytes/version constants. If a
  binary is unavailable or output cannot be bounded reliably, that target stays
  unregistered and the feature remains blocked rather than claiming support.
- **Human status output may drift.** Probe parsers are exact-version code and
  fail closed. A future structured native output can replace a decoder behind
  `EffectiveStateProbePort` without changing orchestration.
- **Global managed projection may expose project assumptions.** The fake adapter
  matrix proves both scopes before concrete adapters register; Codex regression
  proves the project route did not change. The fallback is to keep a target
  observe-only globally, not duplicate a second orchestrator.
- **Gemini trust may be invisible to a non-interactive process.** Absence of
  positive trust evidence is unverified, never trusted. Declared files remain
  observable and status gives the user the native trust/reload next action.
- **OpenCode schema conversion is the least isomorphic mapping.** The codec
  admits only fields with attested equivalents and classifies the rest partial
  or blocked. It never forwards source JSON wholesale.
- **Kiro plugin components and standalone links have different ownership.**
  Resource identity and target-local manifests keep them separate. A collision
  at the same native skill name is an unmanaged/owned conflict, not an
  overwrite or coalescing heuristic.
- **JSON reserialization can change formatting.** Semantic unknown fields are
  preserved; byte formatting is not promised by the native JSON contracts.
  Ownership fingerprints cover only skilltap-owned server values, so unrelated
  formatting changes do not become false ownership evidence.
