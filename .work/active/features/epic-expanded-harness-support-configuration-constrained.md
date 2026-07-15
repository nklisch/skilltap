---
id: epic-expanded-harness-support-configuration-constrained
kind: feature
stage: implementing
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-project-skill-links, epic-expanded-harness-support-declaration-managed]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/kimi-skills.md
  - .research/attestation/kimi-mcp.md
  - .research/attestation/kimi-plugins.md
  - .research/attestation/mistral-skills.md
  - .research/attestation/mistral-mcp.md
  - .research/attestation/kilo-skills.md
  - .research/attestation/kilo-mcp.md
  - .research/attestation/kilo-marketplace.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-15
---

# Configuration-Constrained Adapters for Kimi, Vibe, and Kilo

## Brief

Deliver complete adapters for Kimi Code CLI, Mistral Vibe, and Kilo Code while
preserving their distinct new-session reload, transport/authentication, JSONC,
and configuration-precedence constraints. Each target meets the same
global-and-project skill and MCP admission contract; its limitations appear as
typed capabilities and health evidence rather than being smoothed into generic
support.

The adapters consume shared managed projection and target-local state, preserve
unknown documented native configuration, and classify unsupported transports,
authentication, hooks, agents, or other optional components through the normal
faithful/partial/blocked model. Each target ships with isolated validation and
the common adapter acceptance evidence. This feature does not add target-local
exceptions to the core planner.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: parallel concrete-adapter feature after the registry,
  managed fallback, and project-skill projection foundations.

## Simplification opportunity

- Express reload, transport, and document-format differences as adapter profile
  data and target-owned codecs instead of duplicating reconciliation policy.

## Foundation references

- `docs/VISION.md` — Faithfulness Before Portability, Explicit Loss, deep
  support rather than broad claims.
- `docs/SPEC.md` — registered targets, global/project scopes, managed fallback,
  ownership, drift, and unknown-field preservation.
- `docs/ARCH.md` — typed registry, versioned capability profiles, private native
  DTOs, managed projection, and bounded process/filesystem ports.
- `docs/HARNESS-CONTRACTS.md` — Expanded Target Set, Scope Mapping, MCP Mapping,
  Unknown Harness Versions, and adapter admission acceptance.
- `docs/UX.md` — deterministic target/scope selection, attention states, and
  agent-readable next actions.

## Grounding summary

The prerequisites are implemented or review-ready and this design consumes
their realized contracts rather than their earlier sketches:

- `epic-expanded-harness-support-registry` is `done`. The live registry is
  `TargetRegistry` plus distinct `HarnessAdapter` implementations in
  `crates/harnesses/src/registry.rs`; optional `SkillProjectionPort` and
  `ManagedProjectionPort` accessors keep target behavior out of CLI dispatch.
- `feature-managed-fallback-target-parity` is `done`. The active contract is
  `ManagedProjectionPort::plan(ManagedProjectionContext)`, with one
  `ResolvedSourceCheckout` for apply, source-free removal, adapter-produced
  `ManagedProjectionPlan` writes/manifest/fingerprints, target-local ownership,
  pending-attempt recovery, and the dependency-neutral managed acceptance
  matrix. The superseded acquire/project contract is not used.
- `epic-expanded-harness-support-project-skill-links` is implementation-complete
  and at `review`. Its live project contract validates the canonical complete
  tree at `<project>/.agents/skills/<name>`, derives native destinations through
  `SkillProjectionPort`, and creates a relative per-skill link only when the
  target does not already consume the canonical root. All three targets attest
  `.agents/skills` compatibility, so their project projection is deliberately
  `Canonical`/`NotRequired`, not a copied tree or redundant native-root link.
- `ManagedProjectionContext` and the CLI executor are still project-only by
  name and shape (`project: &AbsolutePath`, `managed_project_lifecycle()` and
  `ManagedProjectLifecyclePort`). That is insufficient for these targets,
  whose lack of deterministic native package lifecycle requires the same
  managed plugin projection at global and project scope.
- The current managed route runs before `configured_native_profile`, so managed
  file mutation is not currently tied to a verified executable version. This
  feature closes that gap with a distinct compiled `managed.projection`
  capability and never treats a runtime probe as mutation authority.
- `AdapterObservationPaths` currently carries roots and labels but not
  adapter-authored health findings, and managed execution verifies bytes rather
  than the harness's post-reload effective state. Kimi's new-session rule,
  Vibe's trusted-project gate, and Kilo's failed/authentication-required states
  therefore need one bounded read-only activation-probe contract shared by
  status and post-apply verification.
- `PlatformPaths` models `HOME`, XDG, Codex, and Claude overrides but not
  `KIMI_CODE_HOME`; Kimi must add that validated environment boundary rather
  than silently assuming `~/.kimi-code`.
- The exact target research establishes admission, paths, precedence, supported
  transport families, Kimi's new-session reload, Vibe's OAuth limitation and
  trust gate, Kilo's JSONC/direct-edit surface, and the absence of deterministic
  native lifecycle commands. It does not preserve exact version output bytes,
  non-interactive effective-state argv/output, complete target wire examples,
  or Kilo's precedence when both documented project filenames exist.

The mapping was direct-read only. Nested agents and peer mechanisms were
explicitly prohibited. The child stories below are durable checkpoints for one
cohesive feature owner, not parallel worker assignments.

## Evidence blocker

The existing source-direct attestations are sufficient to design boundaries but
insufficient to grant mutation authority. Before any of these adapters is
registered with a supported `managed.projection` capability, the first child
must capture, in isolated fixtures, for one installed release of each target:

1. exact version command argv and output decoding;
2. exact global/project MCP document examples and precedence, including Kilo's
   two documented project filenames;
3. a deterministic, bounded, non-interactive effective-state probe that can run
   in a fresh session/project working directory and distinguish loaded,
   reload-required, trust-required, authentication-required, and failed state;
4. round-trip evidence that the chosen Vibe TOML and Kilo JSONC mutation paths
   preserve unrelated and unknown native content.

This is a bounded contract refresh and isolated validation, not a new broad
research campaign. If any target exposes only an interactive UI/slash command
that cannot be driven deterministically without a TTY, or if its exact
write/reload boundary cannot be reproduced, that target's mutation profile
remains unverified and the dependent adapter story is blocked. The
implementation must not invent version literals, parse human text across
versions, or register an observe-only adapter as complete support.

## Design decisions

- **Adapter shape:** implement `KimiAdapter`, `VibeAdapter`, and `KiloAdapter` as
  distinct registry entries. Share only source-plugin extraction and normalized
  portable MCP classification in a private
  `adapters/configuration_constrained/source.rs` module. Native paths, document
  codecs, precedence, probe argv/decoding, and capability profiles remain in
  each target module.
- **No interactive native lifecycle:** Kimi's user-only plugin TUI/slash flow and
  Kilo's sidebar marketplace are observable native state, not deterministic
  mutation APIs. Vibe has no documented package lifecycle. All three return
  `None` from `native_lifecycle()` and use managed projection for global and
  project plugin resources.
- **Version-bounded mutation:** a read-only detection/probe may run on an
  unknown version, but every skill or managed projection write requires a
  verified compiled profile for the exact detected version and concrete scope.
  The profile advertises `component.skill`, `component.mcp`, and
  `managed.projection`; unknown versions downgrade all three to `Unverified`.
  Runtime probes may narrow those capabilities and never create them.
- **Scope-generic managed projection:** evolve the existing port from a project
  path to one concrete `Scope`. The port declares whether it supports that
  scope; orchestration resolves one profile and one checkout, while adapters
  derive their documented roots from `PlatformPaths` plus the scope. Codex
  retains project-only managed fallback; the three new adapters support both.
- **Canonical skill destination:** all three adapters choose `.agents/skills`
  at global and project scope because the attested clients load that portable
  root directly. `SkillProjectionPort` therefore feeds the review-ready project
  service and yields no redundant project link. Native `.kimi-code/skills`,
  `.vibe/skills`, and `.kilo/skills` roots remain observed for precedence and
  unmanaged-conflict evidence; skilltap does not copy a second managed tree
  there.
- **Strict complete-skill reuse:** managed plugin skill components are converted
  to `ValidatedSkillTree`, checked by `validate_agent_skill`, and passed through
  the same adapter compatibility method as standalone project skills before a
  tree write is planned. A required malformed/incompatible skill blocks; an
  optional target-specific component becomes an acknowledged omission.
- **Portable MCP source boundary:** the private source layer accepts complete
  stdio and remote definitions only when command/args/cwd, URL/transport,
  enablement, timeout, tool filters, and credential references are representable.
  Literal credential material is rejected and never enters inventory/state.
  The layer names OAuth explicitly rather than inferring it from arbitrary
  headers.
- **Target transport mapping:** Kimi maps its attested stdio, HTTP, and SSE
  forms. Vibe maps only the exact stdio, HTTP, and streamable-HTTP forms locked
  by its fixture; OAuth-required servers are unsupported. Kilo maps only the
  local/remote forms in its locked JSONC schema. Unsupported optional servers
  produce `ManagedProjection::Omitted`; unsupported required servers return
  `ManagedProjectionError::RequiredUnsupported`, including under `--yes`.
- **Kimi paths/reload:** add validated `KIMI_CODE_HOME` handling with
  `~/.kimi-code` fallback. Global MCP is `<kimi-home>/mcp.json`; project MCP is
  `<project>/.kimi-code/mcp.json`; project entries override equal user names.
  Post-apply verification launches the locked fresh-session probe. A write is
  never reported effective from file presence alone.
- **Vibe document/trust:** global MCP is `~/.vibe/config.toml`; project MCP is
  `<project>/.vibe/config.toml`, which has precedence only in a trusted project.
  A private lossless `VibeConfigDocument` edits named `[[mcp_servers]]` entries
  and preserves unrelated tables, comments, and unknown fields. Trust refusal
  is `trust.required` health, not drift; declared managed bytes remain owned and
  repeat idempotently while status stays attention-required.
- **Kilo JSONC/precedence:** global MCP is
  `${XDG_CONFIG_HOME:-~/.config}/kilo/kilo.jsonc`. A profile-bound private
  resolver selects the documented effective project file (`kilo.jsonc` or
  `.kilo/kilo.jsonc`) using the locked precedence. If an unmanaged
  higher-precedence file would shadow an owned lower-precedence file, mutation
  blocks with `configuration.higher-precedence`; skilltap never writes both.
  `KiloJsoncDocument` performs span/token-preserving edits so comments, trailing
  commas, key order, and unknown fields survive.
- **Declared versus effective state:** file/tree observation remains the
  declared-state source. `ManagedActivationProbe` is a read-only bounded native
  process port used after apply and by status. It returns typed projection
  identities and registered health findings. Reload, trust, auth, or runtime
  failure may leave a correctly applied declared representation with an
  attention-required effective state; it is never mislabeled as drift and is
  not silently rolled back.
- **Finding vocabulary:** add registered `reload.required` and
  `authentication.required` findings; reuse `trust.required`,
  `configuration.higher-precedence`, `native.state.incomplete`, and
  `capability.unverified`. Raw argv, config documents, native output, URLs,
  headers, and error text never enter findings.
- **No UI work:** this is a deterministic CLI/adapter capability. No screen or
  flow surface exists, so the UI fallback is skipped.
- **Review policy:** effective weight is `standard`. Child stories close on
  verification. After all children are done, the feature receives exactly one
  independent feature-level review pass, then receiver adjudication, blocker
  fixes, verification, and completion without a second pass. Design-time
  advisory review was skipped because this invocation forbids nested agents and
  peers; design review is non-blocking.

## Architectural choice

**Chosen — distinct adapters over a scope-generic managed projection and one
private portable-source layer.** The registry remains the only target list.
Core retains normalized evidence and target-local state. Harnesses owns source
readers, exact profile/probe contracts, target paths, and private codecs. CLI
owns scope/profile orchestration, revalidated file/tree execution, post-apply
observation, and rendering. The existing project-skill service consumes each
adapter's `.agents/skills` destination without a target-id branch.

**Rejected — one declarative `FileManagedAdapter` configured with paths and
format flags.** It would make Kimi's session reload, Vibe's trust/OAuth rules,
and Kilo's lossless JSONC/dual-project-file precedence into flags on a generic
codec. That is a universal native format in disguise and makes future changes
risk every target.

**Rejected — invoke interactive plugin/marketplace UI flows.** Kimi and Kilo do
not attest deterministic non-interactive package mutation, and Vibe has no
native package lifecycle. Driving a TUI would violate the product's
non-interactive and version-bounded contracts.

**Rejected — write native skill roots as well as `.agents/skills`.** All three
load the canonical portable root. A second tree creates duplicate ownership and
drift surfaces and bypasses the completed project-link contract.

## Implementation Units

### Unit 1: Lock exact native contracts before authority

**Files**:

- `crates/harnesses/src/adapters/configuration_constrained/contracts.rs` (new)
- `crates/harnesses/tests/fixtures/configuration_constrained/{kimi,vibe,kilo}/`
  (new bounded version/config/probe fixtures)
- focused tests under `crates/harnesses/src/adapters/configuration_constrained/`

**Story**: `epic-expanded-harness-support-configuration-constrained-contract-lock`

```rust
pub(super) struct VerifiedManagedTargetContract {
    pub verified_version: &'static str,
    pub profile_id: &'static str,
    pub version_arguments: &'static [&'static str],
    pub effective_arguments: &'static [&'static str],
    pub document_contract: NativeDocumentContract,
}

pub(super) enum NativeDocumentContract {
    KimiJson,
    VibeToml,
    KiloJsonc { project_precedence: KiloProjectPrecedence },
}

pub(super) enum KiloProjectPrecedence {
    RootThenDotKilo,
    DotKiloThenRoot,
}
```

**Implementation notes**:

- The constants are populated only from source-direct refresh plus isolated
  installed-binary capture. The design intentionally supplies no guessed
  version or argv literal.
- Fixtures contain bounded non-secret documents/output only and state their
  source URL/version in test comments. Authentication values use inert
  references.
- A contract is usable only when the probe is non-interactive, bounded, and
  distinguishes the states listed in the Evidence blocker.

**Acceptance criteria**:

- [ ] Exact version bytes for each target decode to one `NativeVersion`; malformed,
      extra-document, control-character, and unknown-version cases cannot select
      a verified profile.
- [ ] Exact scoped document fixtures pin field names, transport spellings,
      precedence, and unknown-field/comment behavior.
- [ ] Effective probe fixtures distinguish loaded, reload-required,
      trust-required, authentication-required, and failed states without raw
      output crossing the adapter boundary.
- [ ] Failure to lock one target leaves that target explicitly blocked and does
      not weaken or delay independently locked siblings.

---

### Unit 2: Scope-generic, version-gated managed projection

**Files**:

- `crates/core/src/managed_projection.rs`
- `crates/core/src/domain/resource/finding.rs`
- `crates/harnesses/src/managed_projection.rs`
- `crates/harnesses/src/registry.rs`
- `crates/harnesses/src/adapters/codex.rs`
- `crates/cli/src/application.rs`
- `crates/cli/src/application/lifecycle.rs`
- `crates/cli/src/application/execution.rs`
- `crates/cli/src/application/status.rs`
- `crates/cli/src/application/project_skills.rs`

**Story**: `epic-expanded-harness-support-configuration-constrained-projection-scope`

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ManagedProjectionIdentity {
    Skill(RelativeArtifactPath),
    Mcp(NativeId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedActivationState {
    Loaded,
    ReloadRequired,
    TrustRequired,
    AuthenticationRequired,
    Failed,
}

pub struct ManagedActivationObservation {
    pub projections: BTreeMap<ManagedProjectionIdentity, ManagedActivationState>,
    pub findings: Vec<ObservationFinding>,
}

pub struct ManagedActivationRequest<'a> {
    pub target: &'a HarnessId,
    pub scope: &'a Scope,
    pub action: ManagedLifecycleKind,
    pub expected: &'a [ManagedProjection],
}

pub trait ManagedActivationProbe: Sync {
    fn arguments(&self, request: &ManagedActivationRequest<'_>) -> Vec<OsString>;
    fn working_directory(&self, scope: &Scope) -> Option<AbsolutePath>;
    fn decode(
        &self,
        request: &ManagedActivationRequest<'_>,
        stdout: &[u8],
        stderr: &[u8],
        status: ExitStatus,
        limits: JsonLimits,
    ) -> Result<ManagedActivationObservation, ManagedActivationError>;
}

pub trait ManagedProjectionPort: Sync {
    fn supports_scope(&self, scope: &Scope) -> bool;
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError>;
    fn activation_probe(&self) -> Option<&dyn ManagedActivationProbe>;
}

pub struct ManagedProjectionContext<'a> {
    pub target: &'a HarnessId,
    pub scope: &'a Scope,
    pub paths: &'a PlatformPaths,
    // existing resource/request/input/prior/ack/filesystem/limit fields remain
}
```

**Implementation notes**:

- Replace `project` with exact `scope`; adapters derive roots. Rename the
  misleading CLI execution types to `ManagedProjectionExecutionPort`,
  `ManagedProjectionExecutionEntry`, `ManagedProjectionFileWrite`, and
  `ManagedProjectionTreeWrite` while retaining revalidation/rollback behavior.
- Remove `HarnessAdapter::managed_project_lifecycle()`. Routing uses
  `adapter.managed_projection().filter(|port| port.supports_scope(scope))`.
  Codex returns true only for project scope; new ports return true for both.
- Add `configured_adapter_profile`, independent of `native_lifecycle()`, which
  resolves/detects once and checks a named capability. Managed mutation checks
  `managed.projection`; standalone skill mutation checks `component.skill`.
- Run activation as a bounded read-only postcondition after declared bytes are
  applied and journaled. Effective attention does not rewrite or discard
  correct declared state. Status invokes the same decoder and merges its
  registered findings into `HarnessObservation`.
- `AdapterObservationPaths` gains `findings: Vec<ObservationFinding>`; status
  appends profile-unverified evidence rather than replacing adapter findings.

**Acceptance criteria**:

- [ ] Codex project managed regression remains unchanged; Codex global still
      prefers its verified native lifecycle.
- [ ] A fake managed adapter executes identical install/update/remove flows at
      global and project scope through one port, preserving sibling target state.
- [ ] Unknown versions and unsupported scopes plan no writes; a runtime probe
      cannot widen an unverified compiled profile.
- [ ] Project skill install/link and managed projection both block before write
      when the adapter's required compiled capability is unverified.
- [ ] Reload/trust/auth findings produce attention-required effective health
      without being labeled drift; immediate repeats do not rewrite files.
- [ ] Every write still binds to an operation, revalidates under the lock, rolls
      back only captured identities, and reports residuals if restoration fails.

---

### Unit 3: Private portable source and compatibility planner

**Files**:

- `crates/harnesses/src/adapters/configuration_constrained/source.rs` (new)
- `crates/harnesses/src/adapters/configuration_constrained/mod.rs` (new)
- `crates/harnesses/src/adapters/mod.rs`

**Story**: `epic-expanded-harness-support-configuration-constrained-source`

```rust
pub(super) struct SelectedPortablePlugin {
    pub tree: ArtifactTree,
    pub declarations: Vec<ComponentDeclaration>,
    pub mcp: BTreeMap<NativeId, PortableMcpServer>,
}

pub(super) enum PortableMcpServer {
    Stdio {
        command: String,
        args: Vec<String>,
        environment: BTreeMap<String, CredentialReference>,
        cwd: Option<String>,
        enabled: bool,
        timeout_ms: Option<u64>,
        tools: Option<BTreeSet<String>>,
    },
    Remote {
        transport: PortableRemoteTransport,
        url: String,
        headers: BTreeMap<String, CredentialReference>,
        authentication: AuthenticationRequirement,
        enabled: bool,
        timeout_ms: Option<u64>,
        tools: Option<BTreeSet<String>>,
    },
}

pub(super) enum PortableRemoteTransport {
    Http,
    Sse,
    StreamableHttp,
}

pub(super) enum AuthenticationRequirement {
    None,
    StaticReferences,
    OAuth,
}

pub(super) fn load_selected_plugin(
    context: &ManagedProjectionContext<'_>,
) -> Result<Option<SelectedPortablePlugin>, ManagedProjectionError>;
```

**Implementation notes**:

- Resolve the exact selector from the caller checkout using private validating
  Codex/Claude catalog codecs; direct plugin roots are accepted only when the
  explicit source itself has one recognized manifest. Never search recursively.
- Reuse `CodexPluginGraphReader`/`ClaudePluginGraphReader`,
  `ComponentDeclaration`, `ValidatedSkillTree`, and `validate_agent_skill`.
- Snapshot the selected complete tree once. Reject symlinks and enforce the
  existing tree/JSON limits.
- Credential values must be environment/reference expressions. Literal secret
  material is unsupported; no raw server object enters state or findings.
- The module classifies components but does not choose target support. Each
  target codec maps the normalized server or returns omitted/blocked evidence.

**Acceptance criteria**:

- [ ] Exact selected local/Git source roots produce one deterministic component
      graph and complete skill trees; ambiguous, escaping, malformed, or
      recursively discovered candidates are rejected.
- [ ] Stdio/remote transport, auth requirement, enablement, timeout, and tool
      filters survive normalization without credential values entering state.
- [ ] Required malformed/incompatible components block; optional unsupported
      components remain itemized for normal acknowledgment.
- [ ] No Kimi, Vibe, or Kilo document/path vocabulary leaks into core or CLI.

---

### Unit 4: Kimi Code adapter

**Files**:

- `crates/core/src/runtime/error.rs` and `crates/core/src/runtime/paths.rs`
- `crates/harnesses/src/adapters/configuration_constrained/kimi.rs` (new)
- `crates/harnesses/src/adapters/configuration_constrained/kimi_projection.rs`
  (new, private codec/port)
- `crates/harnesses/src/registry.rs`

**Story**: `epic-expanded-harness-support-configuration-constrained-kimi`

```rust
pub struct KimiAdapter;

struct KimiMcpDocument {
    root: serde_json::Map<String, serde_json::Value>,
}

impl KimiMcpDocument {
    fn parse(bytes: Option<&[u8]>, limits: JsonLimits) -> Result<Self, KimiCodecError>;
    fn upsert(&mut self, id: &NativeId, server: &PortableMcpServer)
        -> Result<(), KimiCodecError>;
    fn remove(&mut self, id: &NativeId) -> Result<(), KimiCodecError>;
    fn managed_fingerprint(&self, ids: &BTreeSet<NativeId>) -> Option<Fingerprint>;
    fn encode(self) -> Result<Option<Vec<u8>>, KimiCodecError>;
}
```

**Implementation notes**:

- Add `EnvironmentVariable::KimiCodeHome` and
  `PlatformPaths::kimi_code_home()` with validated absolute override and
  `~/.kimi-code` fallback; include it in explicit native process environments.
- Register id `kimi`, display `Kimi Code`, managed distribution, no native
  lifecycle. Profile values/probe decoder come only from Unit 1.
- `SkillProjectionPort` returns `~/.agents/skills` globally and
  `<project>/.agents/skills` for project scope; compatibility consumes strict
  Agent Skills validation.
- Observe both portable and Kimi-native skill roots plus scoped `mcp.json`.
  Project names shadow global names; duplicate evidence is typed.
- The private JSON codec mutates only `mcpServers.<managed-id>`, preserves
  unrelated top-level/server fields, and encodes Kimi's exact stdio/HTTP/SSE,
  timeout, enablement, and tool-filter shapes.
- The activation probe always starts a fresh session in project cwd when
  project-scoped, then verifies expected identities; stale-session evidence is
  `reload.required`, never success inferred from bytes.

**Acceptance criteria**:

- [ ] Known version supports both scopes; unknown version is observe-only.
- [ ] KIMI home override/default, both skill roots, both MCP files, and project
      precedence are observed without host-state access.
- [ ] Managed install/update/remove preserves unknown JSON and unmanaged
      servers; same-name project override is effective after a fresh session.
- [ ] Stdio, HTTP, and SSE map faithfully; unsupported optional components are
      acknowledged omissions and required unsupported components block.
- [ ] Every mutation immediately repeats with no file/tree/state change.

---

### Unit 5: Mistral Vibe adapter

**Files**:

- `Cargo.toml` and `crates/harnesses/Cargo.toml` — add `toml_edit` for the
  selected lossless native TOML codec
- `crates/harnesses/src/adapters/configuration_constrained/vibe.rs` (new)
- `crates/harnesses/src/adapters/configuration_constrained/vibe_projection.rs`
  (new, private codec/port)
- `crates/harnesses/src/registry.rs`

**Story**: `epic-expanded-harness-support-configuration-constrained-vibe`

```rust
pub struct VibeAdapter;

struct VibeConfigDocument {
    document: toml_edit::DocumentMut,
}

impl VibeConfigDocument {
    fn parse(bytes: Option<&[u8]>) -> Result<Self, VibeCodecError>;
    fn upsert(&mut self, id: &NativeId, server: &PortableMcpServer)
        -> Result<(), VibeCodecError>;
    fn remove(&mut self, id: &NativeId) -> Result<(), VibeCodecError>;
    fn managed_fingerprint(&self, ids: &BTreeSet<NativeId>) -> Option<Fingerprint>;
    fn encode(self) -> Option<Vec<u8>>;
}
```

**Implementation notes**:

- Register id `vibe`, display `Mistral Vibe`, managed distribution, no native
  lifecycle. Native root is `~/.vibe`.
- Use `.agents/skills` at both scopes while observing native `.vibe/skills`
  roots for conflicts/precedence.
- Edit only named `[[mcp_servers]]` entries in user/project `config.toml`.
  Preserve comments, ordering, unrelated tables, enable/disable filters, and
  unknown keys. Fingerprint managed entries, not the whole user document.
- Map only the fixture-locked stdio/HTTP/streamable-HTTP forms. OAuth is never
  downgraded to static headers: optional OAuth servers are omitted with a
  target-specific consequence; required OAuth servers block even with `--yes`.
- Project probe runs in the project cwd. An untrusted directory yields
  `trust.required`; the correct declared document remains owned and is not
  mislabeled drift or repeatedly rewritten.

**Acceptance criteria**:

- [ ] Known version supports global/project scopes; unknown version is
      observe-only.
- [ ] User/project config precedence and trusted/untrusted outcomes match the
      locked contract.
- [ ] Lossless TOML edits preserve comments, unknown fields, unrelated server
      tables, skill filters, and byte-stable no-op repeats.
- [ ] Supported transports map exactly; OAuth and unsupported transport
      consequences follow optional/required policy.
- [ ] Removal deletes only owned named tables and leaves unmanaged native state
      intact.

---

### Unit 6: Kilo Code adapter

**Files**:

- `crates/harnesses/src/adapters/configuration_constrained/kilo.rs` (new)
- `crates/harnesses/src/adapters/configuration_constrained/kilo_projection.rs`
  (new, private JSONC codec/port)
- `crates/harnesses/src/registry.rs`

**Story**: `epic-expanded-harness-support-configuration-constrained-kilo`

```rust
pub struct KiloAdapter;

struct KiloJsoncDocument {
    source: Vec<u8>,
    root: JsoncObject,
}

impl KiloJsoncDocument {
    fn parse(bytes: Option<&[u8]>, limits: JsonLimits) -> Result<Self, KiloCodecError>;
    fn upsert(&mut self, id: &NativeId, server: &PortableMcpServer)
        -> Result<(), KiloCodecError>;
    fn remove(&mut self, id: &NativeId) -> Result<(), KiloCodecError>;
    fn managed_fingerprint(&self, ids: &BTreeSet<NativeId>) -> Option<Fingerprint>;
    fn encode(self) -> Result<Option<Vec<u8>>, KiloCodecError>;
}

struct KiloDocumentResolver;

impl KiloDocumentResolver {
    fn resolve(
        paths: &PlatformPaths,
        scope: &Scope,
        filesystem: &dyn ConfinedFileSystem,
        precedence: KiloProjectPrecedence,
    ) -> Result<KiloDocumentLocation, KiloCodecError>;
}
```

**Implementation notes**:

- Register id `kilo`, display `Kilo Code`, managed distribution, no native
  lifecycle. Global native root is `<config-home>/kilo`.
- Use `.agents/skills` at both scopes while observing `.kilo/skills` roots.
- Resolve the one effective project JSONC file using Unit 1's locked
  precedence. If a higher-precedence unmanaged document shadows an owned lower
  document, block and preserve both; never merge/write both.
- The JSONC codec is token/span preserving. It patches only the exact managed
  MCP object members and retains comments, trailing commas, quote style where
  unchanged, key order, and unknown fields. A serde JSON round-trip is not an
  acceptable implementation.
- Activation decoding maps Kilo's loaded, failed, and authentication-required
  runtime states to typed health. Authentication material remains native and
  outside skilltap state.

**Acceptance criteria**:

- [ ] Known version supports both scopes; unknown version is observe-only.
- [ ] Global path, both project candidates, exact precedence, and shadowing
      conflicts match the locked contract.
- [ ] JSONC install/update/remove preserves all unrelated bytes and comments;
      managed fingerprints ignore unrelated formatting changes but detect owned
      entry drift.
- [ ] Supported local/remote transports map exactly; auth-required and failed
      effective state remain attention-required rather than false drift.
- [ ] Immediate repeats are byte-, inode-, plan-, and target-state no-ops.

---

### Unit 7: Integrated adapter acceptance

**Files**:

- `crates/test-support/src/harness_profile.rs`
- `crates/test-support/src/managed_acceptance.rs`
- `crates/test-support/src/integration.rs`
- `crates/harnesses/src/adapters/mod.rs` tests
- `crates/harnesses/tests/detection.rs`
- `crates/cli/src/application/tests.rs`
- `crates/cli/tests/compiled_binary.rs`

**Story**: `epic-expanded-harness-support-configuration-constrained-acceptance`

**Implementation notes**:

- Extend fake profiles with scope-specific skill/MCP locations and exact
  activation responses; do not add a target switch outside profile
  constructors/runner registration.
- Run `acceptance_matrix`, `managed_acceptance_matrix`, and compiled CLI cases
  for `kimi`, `vibe`, and `kilo` inside isolated homes/config roots/projects.
- Exercise the review-ready canonical project-skill path: all three adapters
  produce `projection=not_required` and no native-root duplicate.
- Snapshot native files without following links and never run real operator
  binaries in ordinary tests.

**Acceptance criteria**:

- [ ] Registry/help/config enablement list all five supported concrete targets
      in stable order without another hard-coded CLI list.
- [ ] Each target passes known/unknown detection, both scopes, complete skill
      discovery, MCP merge/precedence, effective probe, drift, removal,
      target-local state preservation, pending recovery, and immediate-repeat
      idempotency.
- [ ] Kimi proves new-session visibility; Vibe proves trusted and untrusted
      project outcomes plus OAuth blocking; Kilo proves JSONC preservation,
      dual-path precedence, failed/auth-required health, and shadow conflict.
- [ ] Optional unsupported hook/agent/transport is itemized and acknowledgment-
      gated; required unsupported remains blocked under `--yes`.
- [ ] Plain and JSON outputs derive from the same typed outcome and expose no
      raw native config, output, secret, argv, or dynamic parser text.
- [ ] `cargo test --workspace --all-targets`, Clippy with warnings denied,
      formatting, and `git diff --check` pass before feature review.

## Implementation Order

1. `epic-expanded-harness-support-configuration-constrained-contract-lock` —
   evidence gate, `depends_on: []`.
2. `epic-expanded-harness-support-configuration-constrained-projection-scope`
   — scope/profile/activation contract, `depends_on: [contract-lock]`.
3. `epic-expanded-harness-support-configuration-constrained-source` — private
   source and compatibility planner, `depends_on: [projection-scope]`.
4. `epic-expanded-harness-support-configuration-constrained-kimi` — Kimi
   adapter, `depends_on: [source]`.
5. `epic-expanded-harness-support-configuration-constrained-vibe` — Vibe
   adapter, `depends_on: [source]`.
6. `epic-expanded-harness-support-configuration-constrained-kilo` — Kilo
   adapter, `depends_on: [source]`.
7. `epic-expanded-harness-support-configuration-constrained-acceptance` —
   integrated closure, `depends_on: [kimi, vibe, kilo]`.

The three target stories are independent after the shared source contract, but
one feature owner should normally implement them sequentially in the research
order Kimi → Vibe → Kilo to reuse discoveries without overlapping registry and
test-runner writes. They are not default parallel agent assignments.

`work-view --blocking` was run for every new story id before dependencies were
written; no existing dependents or cycles were reported.

## Simplification

- Replace project-only managed lifecycle naming/dispatch with one scope-generic
  execution path; do not add a separate global orchestrator.
- Remove the duplicate `managed_project_lifecycle()` boolean and derive scope
  eligibility from the managed port itself.
- Add one profile detector usable without a native lifecycle port rather than
  bypassing version authority for managed writes.
- Use `.agents/skills` directly for all three targets; no copied native skill
  trees, link manifests, or target-id branches in project skill planning.
- Keep one private portable source parser for this family; do not copy plugin
  graph/MCP normalization into three codecs and do not promote it to a public
  universal plugin format before another family earns that abstraction.
- Fingerprint only managed document entries, not whole user-authored config
  files, eliminating false drift from comments and unrelated fields.
- Reuse target-local state, pending-attempt recovery, operation binding,
  root-confined writes, rollback, and the acceptance matrix unchanged.
- Do not add native marketplace lifecycle wrappers for interactive-only Kimi or
  Kilo surfaces.

No standalone cleanup/refactor story is warranted. Every rename/removal is
coupled to the behavior unit that proves its replacement.

## Testing

- **Contract fixture tests:** exact version, document, precedence, and activation
  output examples. Protect the mutation-authority boundary and prevent guessed
  profile widening.
- **Shared interface tests:** global/project managed dispatch, unknown-version
  blocking, scope support, activation finding registration, target-local state,
  and Codex project regression. Protect cross-unit seams.
- **Portable source tests:** explicit source selection, confinement, complete
  trees, normalized transports/auth/references, and optional/required
  classification. Protect compatibility semantics.
- **Codec regression tests:** Kimi unknown JSON, Vibe lossless TOML, Kilo JSONC
  comments/trailing commas/dual-path precedence, plus owned-entry drift and
  removal. Protect user-authored native documents.
- **Compiled acceptance:** both scopes, project canonical skills, reload/trust/
  auth attention, output parity, partial policy, recovery, and immediate repeat.
  Protect user-visible support claims.
- **Test removals/updates:** rename project-only managed execution assertions as
  the contract becomes scope-generic; retain Codex semantics and all low-level
  filesystem race tests. Do not add getter tests, snapshots of incidental
  formatting, or one test per parser branch.

## Risks

- **Blocking evidence gap:** exact versions, deterministic effective probes, and
  full wire examples are not in the current substrate. Unit 1 is a hard gate.
  If a deterministic probe is unavailable, the affected target cannot honestly
  meet the admission contract and remains blocked rather than shipping
  file-presence-only support.
- **Lossless Kilo mutation:** JSONC editing can easily destroy comments or patch
  the wrong object. The fallback is to keep Kilo observe-only; falling back to
  `serde_json`, writing both project files, or a UI/cache mutation is not
  acceptable.
- **Vibe trust versus ownership:** a correct declared config can be inactive in
  an untrusted directory. State records the owned declared representation;
  ephemeral effective observation carries trust health. Conflating them would
  cause destructive rewrites or false healthy status.
- **Kimi session freshness:** a status command run in an existing process cannot
  prove a new-session load. The locked activation request must create a fresh
  process/session. If it cannot, mutation remains unverified.
- **Scope generalization blast radius:** renaming/generalizing the managed
  executor touches Codex's proven project path. Codex regression and the fake
  two-scope adapter land before any new target registration; the fallback is to
  stop before registry expansion, not maintain two orchestrators.
- **Portable-source overreach:** source plugins may contain target-specific
  components not represented by the private normalized subset. They stay
  explicit optional omissions or required blockers; the source layer never
  asserts universal plugin equivalence.
- **Shared canonical skill collisions:** plugin-owned and standalone skills can
  request the same `.agents/skills/<name>` destination. Existing ownership and
  fingerprint checks must report the conflict and preserve the incumbent; no
  adapter-specific overwrite rule is introduced.

## Pre-mortem

- **Riskiest assumption:** each harness has a deterministic read-only probe that
  can verify effective state after its required reload/trust boundary. If false,
  complete mutation support is not feasible under the product contract.
- **Production failure condition:** a codec writes valid-looking config but a
  higher-precedence document, stale session, trust gate, auth flow, or transport
  mismatch prevents load while skilltap records healthy state.
- **Mitigation:** profile-bound paths/codecs, managed-entry fingerprints, one
  fresh activation probe, and declared/effective separation. Health remains
  attention-required until effective evidence agrees.
- **Fallback:** retain observation and capability diagnostics but do not expose
  `managed.projection` as supported or call the adapter complete.
- **Least certain area:** exact Kilo dual-file precedence and the non-interactive
  probe surfaces for all three. Unit 1 isolates those uncertainties before
  shared or target mutation code.
