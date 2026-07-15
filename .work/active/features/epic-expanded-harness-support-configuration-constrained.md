---
id: epic-expanded-harness-support-configuration-constrained
kind: feature
stage: done
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-project-skill-links, epic-expanded-harness-support-declaration-managed]
release_binding: 3.1.0
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

Deliver exact-profile adapters for Kimi Code CLI `1.48.0`, Mistral Vibe
`2.19.1`, and Kilo Code `7.4.7` under the relaxed component/scope admission
contract. Each target admits only its documented complete-skill and MCP
surfaces; unsupported scopes and adjacent plugin behavior remain explicitly
blocked. Kimi's global MCP declaration is the only MCP mutation surface;
project MCP is unsupported and no MCP lifecycle/status/auth command is invoked.
Vibe and Kilo use declaration-managed, lossless configuration edits while
leaving effective state unverified and avoiding interactive or side-effectful
probes. Their limitations appear as typed capabilities and health evidence
rather than being smoothed into generic support.

The adapters consume shared managed projection and target-local state, preserve
unrelated documented native configuration, and classify unsupported transports,
authentication, hooks, agents, or other optional components through the normal
faithful/partial/blocked model. Each target ships with isolated validation and
the common adapter acceptance evidence. This feature does not add target-local
exceptions to the core planner.

## Review result

### Standard-pass adjudication — 2026-07-15

The receiver confirmed and fixed all three completed STANDARD review findings;
none remains a current-cycle blocker:

1. **Vibe `cwd` boundary** — Vibe 2.19.1 no longer emits a source stdio
   `cwd` that is not attested by its release contract. The codec rejects it,
   and the existing requiredness path classifies it as an explicit optional
   omission/partial outcome or a required blocker. The compiled `--yes` case
   proves that the faithful skill may proceed only with the MCP omission
   disclosed and no Vibe config containing silently rewritten semantics.
2. **Kilo schema admission** — the static top-level key gate now accepts only
   `mcp` and `username`, the two keys directly evidenced by the locked 7.4.7
   contract and release-sensitive capture. Previously listed but ungrounded
   settings fail closed, with codec tests documenting both accepted evidence
   and conservative rejection.
3. **No-probe compiled evidence** — the constrained Kimi/Vibe/Kilo compiled
   acceptance now requires every known and unknown profile invocation log to
   contain exact `--version` calls only. No native MCP, auth, config, UI, or
   browser command can satisfy the fixture.

The contract-lock story now records the realized inline fixture layout rather
than a nonexistent standalone fixture tree. Verification after adjudication:
focused Vibe/Kilo codec tests, focused constrained compiled tests, the compiled
Vibe `cwd` boundary test, `cargo test --workspace --all-targets` (734 passed),
strict `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo fmt --all -- --check`, and `git diff --check` all pass.

Effective review weight is `standard`: one completed review pass, receiver
adjudication, blocker fixes, and verification, closed without a second review.
The caller prohibited nested-agent and peer paths; no independent second pass
was run. Residual unsupported surfaces remain explicit below and are not
approval gaps.

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
- `AdapterObservationPaths` carries roots and labels, while managed execution
  verifies bytes without claiming harness activation. The relaxed amendment
  intentionally does not add an effective probe for this family: Kimi's
  project MCP is unsupported, Vibe has no safe non-TTY MCP observer, and Kilo's
  documented probes create native state. Their declared/effective separation
  remains explicit in status.
- `PlatformPaths` models `HOME`, XDG, Codex, and Claude overrides but not
  `KIMI_SHARE_DIR` or `VIBE_HOME`; the exact target paths add those validated
  environment boundaries rather than assuming operator locations.
- The exact target research establishes admission, paths, precedence, supported
  transport families, Kimi's new-session reload, Vibe's OAuth limitation and
  trust gate, Kilo's JSONC/direct-edit surface, and the absence of deterministic
  native lifecycle commands. It does not preserve exact version output bytes,
  non-interactive effective-state argv/output, complete target wire examples,
  or Kilo's precedence when both documented project filenames exist.

The mapping was direct-read only. Nested agents and peer mechanisms were
explicitly prohibited. The child stories below are durable checkpoints for one
cohesive feature owner, not parallel worker assignments.

## Relaxed contract amendment — 2026-07-15

The earlier evidence blocker required deterministic effective-state probes for
all three targets. The updated foundation deliberately removes that requirement
where the native product cannot provide a safe observation boundary. This is a
narrow amendment, not a weakening of mutation safety:

- Kimi `1.48.0` is admitted for complete skills on exact documented roots and
  for global MCP declarations at `~/.kimi/mcp.json`, relocated by
  `KIMI_SHARE_DIR`. Project MCP is `Unsupported`. Production never invokes
  `mcp list`, `mcp test`, `mcp auth`, a TUI, or a browser/auth flow.
- Vibe `2.19.1` is admitted for complete skills and user/trusted-project TOML
  declarations. Skilltap patches only the selected `[[mcp_servers]]` entries
  through a lossless syntax-preserving codec. Static credentials and
  references are the admitted subset; OAuth is `Unsupported` because the
  official web contract contradicts the release implementation. No `/mcp`,
  TUI, LLM turn, trust approval, or effective-state probe runs in production.
- Kilo `7.4.7` is admitted for complete skills and valid global/effective
  project JSON/JSONC declarations through targeted token-preserving edits.
  Invalid unknown schema keys and conflicting locations block. Production
  never invokes `debug config`, `mcp list`, or `mcp auth`, and observation
  creates no native database, cache, `.kilo`, or `.gitignore`.

The declaration contract now proves owned bytes, lossless preservation,
conflict/drift detection, rollback, and repeat-idempotence. It does not claim
that a declaration loaded, activated, passed trust, or authenticated. Vibe's
project trust and all three targets' effective state remain unverified; no
approval is stored or inferred. Foreground `--yes` acknowledges only the
reported declaration/effective consequence. Daemon runs leave these operations
pending. A target-specific scope/component capability is `Unsupported` rather
than inferred from an explicit invocation override or from an adjacent native
surface.

The exact release/version/document/probe fixtures recorded by the contract-lock
story remain useful as negative evidence and codec fixtures. They no longer
block independently safe declaration-managed siblings merely because a
side-effect-free effective probe is unavailable.

## Design decisions

- **Adapter shape:** implement `KimiAdapter`, `VibeAdapter`, and `KiloAdapter` as
  distinct registry entries. Share only source-plugin extraction and normalized
  portable MCP classification in a private
  `adapters/configuration_constrained/source.rs` module. Native paths, document
  codecs, precedence, and capability profiles remain in each target module.
  No activation-probe port is implemented for this family because the updated
  contract explicitly forbids production probes.
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
  service and yields no redundant project link. Native `.kimi/skills`,
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
- **Kimi paths/reload:** add validated `KIMI_SHARE_DIR` handling with
  `~/.kimi` fallback. Global MCP is `<kimi-share-dir>/mcp.json`; project MCP
  is `Unsupported`. No MCP status, connection, auth, or fresh-session probe
  is launched by skilltap. A write is declaration-managed and never reported
  effective from file presence alone.
- **Vibe document/trust:** global MCP is `~/.vibe/config.toml`; project MCP is
  `<project>/.vibe/config.toml`, selected only by the documented trusted-project
  layer. A private lossless `VibeConfigDocument` edits named
  `[[mcp_servers]]` entries and preserves unrelated tables, comments, ordering,
  and unknown representable values. Trust is not approved or probed by
  skilltap; declared managed bytes remain owned and repeat idempotently while
  effective state stays unverified and attention-required.
- **Kilo JSONC/precedence:** global MCP is
  `${XDG_CONFIG_HOME:-~/.config}/kilo/kilo.jsonc`. A profile-bound private
  resolver selects the documented effective project file (`kilo.jsonc` or
  `.kilo/kilo.jsonc`) using the locked precedence. If an unmanaged
  higher-precedence file would shadow an owned lower-precedence file, mutation
  blocks with `configuration.higher-precedence`; skilltap never writes both.
  Invalid unknown schema keys also block. `KiloJsoncDocument` performs
  token/span-preserving edits so comments, trailing commas, key order, and
  unrelated valid fields survive. No Kilo debug/config/list/auth command is
  used for effective observation.
- **Declared versus effective state:** file/tree observation remains the
  declared-state source. This family exposes no activation probe: Kimi's
  forbidden MCP commands, Vibe's TUI/LLM-only status, and Kilo's side-effectful
  probes cannot become production observation ports. Managed writes remain
  declared and effective-unverified; trust, auth, reload, or runtime state is
  never inferred from bytes and is never mislabeled as drift.
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

- `crates/harnesses/src/adapters/configuration_constrained/{common,source}.rs`
- focused target codec tests in `crates/harnesses/src/adapters/{kimi,vibe,kilo}.rs`
- compiled acceptance fixtures in `crates/cli/tests/compiled_binary.rs`

**Story**: `epic-expanded-harness-support-configuration-constrained-contract-lock`

```rust
pub(super) struct VerifiedManagedTargetContract {
    pub verified_version: &'static str,
    pub profile_id: &'static str,
    pub version_arguments: &'static [&'static str],
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
- The relaxed contract is usable when declaration ownership, source bounds,
  lossless codecs, and explicit unsupported/effective-unverified outcomes are
  bounded. No effective probe is required or registered for this family.

**Acceptance criteria**:

- [ ] Exact version bytes for each target decode to one `NativeVersion`; malformed,
      extra-document, control-character, and unknown-version cases cannot select
      a verified profile.
- [ ] Exact scoped document fixtures pin field names, transport spellings,
      precedence, and unknown-field/comment behavior.
- [x] No production process/auth/UI probe is registered; effective load,
      reload, trust, authentication, and failure remain unverified/pending
      rather than crossing the declaration boundary.
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

pub trait ManagedProjectionPort: Sync {
    fn plan(
        &self,
        context: &ManagedProjectionContext<'_>,
    ) -> Result<ManagedProjectionPlan, ManagedProjectionError>;
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
- Remove `HarnessAdapter::managed_project_lifecycle()`. Routing uses one
  explicit `Scope` and the existing managed projection port; Codex retains its
  native-lifecycle precedence while new ports serve both scopes.
- Add `configured_adapter_profile`, independent of `native_lifecycle()`, which
  resolves/detects once and checks a named capability. Managed mutation checks
  `managed.projection`; standalone skill mutation checks `component.skill`.
- Declaration-managed targets do not register an activation probe. Status
  reports declared ownership and effective-unverified/pending evidence without
  rewriting correct declared state.
- `AdapterObservationPaths` retains target-authored roots/labels; status never
  invents native findings from file presence.

**Acceptance criteria**:

- [ ] Codex project managed regression remains unchanged; Codex global still
      prefers its verified native lifecycle.
- [ ] A fake managed adapter executes identical install/update/remove flows at
      global and project scope through one port, preserving sibling target state.
- [ ] Unknown versions and unsupported scopes plan no writes; a runtime probe
      cannot widen an unverified compiled profile.
- [ ] Project skill install/link and managed projection both block before write
      when the adapter's required compiled capability is unverified.
- [x] Reload/trust/auth are not inferred from declaration bytes; the target
      remains effective-unverified/pending without being labeled drift, and
      immediate repeats do not rewrite files.
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

- Add `EnvironmentVariable::KimiShareDir` and
  `PlatformPaths::kimi_share_dir()` with validated absolute override and
  `~/.kimi` fallback.
- Register id `kimi`, display `Kimi Code CLI`, managed distribution, no native
  lifecycle. The exact `1.48.0` profile grants global MCP `Unverified` and
  project MCP `Unsupported`.
- `SkillProjectionPort` returns `.agents/skills` globally and
  `<project>/.agents/skills` for project scope; compatibility consumes strict
  Agent Skills validation.
- Observe both portable and Kimi-native skill roots. Global MCP is only
  `<kimi-share-dir>/mcp.json`; project MCP is rejected before any project MCP
  read or write.
- The private JSON codec mutates only `mcpServers.<managed-id>`, preserves
  unrelated top-level/server fields, and maps only Kimi's exact stdio/HTTP/SSE
  forms. OAuth, streamable HTTP, and literal credentials fail closed.
- No Kimi MCP command, UI, browser, auth flow, or fresh-session probe runs.

**Acceptance criteria**:

- [x] Known/unknown version authority, both skill scopes, global MCP, and
      project MCP `Unsupported` are compiled and tested.
- [x] Managed install/update/remove preserves unknown JSON and unmanaged
      servers without a project MCP path.
- [x] Stdio, HTTP, and SSE map faithfully; OAuth, streamable HTTP, and literal
      credentials are rejected; optional/required policy is preserved.
- [x] Every mutation immediately repeats with no file/tree/state change.

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
- Map only stdio, HTTP, and streamable-HTTP. OAuth and SSE are explicitly
  unsupported; optional/required consequences use the normal partial/block
  policy.
- Project trust, reload, and effective load remain unverified. No `/mcp`, TUI,
  LLM, browser, trust-approval, or effective-state probe runs.

**Acceptance criteria**:

- [x] Known/unknown version authority and global/project declaration scopes
      match the locked contract; trust remains unverified.
- [x] Lossless TOML edits preserve comments, unknown fields, unrelated server
      tables, filters, and byte-stable no-op repeats.
- [x] Supported transports map exactly; OAuth and SSE follow optional/required
      unsupported policy.
- [x] Removal deletes only owned named tables and leaves unmanaged native state
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
- Resolve the one valid global/project JSON/JSONC document. Invalid unknown
  schema keys and conflicting locations block; an unmanaged higher-precedence
  document is never merged or overwritten.
- The JSONC codec is token/span preserving. It patches only the exact managed
  MCP object members and retains comments, trailing commas, quote style where
  unchanged, key order, and unknown fields. A serde JSON round-trip is not an
  acceptable implementation.
- Effective load and authentication remain unverified. No Kilo debug config,
  MCP list/auth command, cache, database, `.kilo`, or `.gitignore` is created
  by observation.

**Acceptance criteria**:

- [x] Known/unknown version authority, global/project paths, precedence, and
      shadow conflicts match the locked contract.
- [x] JSONC install/update/remove preserves unrelated bytes/comments; managed
      fingerprints ignore unrelated formatting and detect owned-entry drift.
- [x] Supported local/remote transports map exactly; OAuth, non-HTTP transport,
      unknown schema, and invalid-document outcomes fail closed.
- [x] Immediate repeats are byte-, plan-, and target-state no-ops.

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

- [x] Registry/help/config enablement list `kimi`, `vibe`, and `kilo` through
      the canonical registry without another production target list.
- [x] Each target passes known/unknown detection, both scopes, complete skill
      discovery, declaration precedence, no-ack/`--yes`, daemon pending,
      conflict, removal, target-local state preservation, and immediate-repeat
      idempotency.
- [x] Kimi proves global-only MCP, project MCP `Unsupported`, static/OAuth and
      unsupported-transport rejection, and no-probe command sentinels; Vibe
      proves lossless TOML edits and OAuth/SSE rejection; Kilo proves JSONC
      preservation, precedence/shadowing, unknown-schema rejection, and no
      probe commands.
- [x] Optional unsupported components are itemized and acknowledgment-gated;
      required unsupported components remain blocked under `--yes`.
- [x] Plain and JSON outputs derive from the same typed outcome and expose no
      raw native config, output, secret, argv, or dynamic parser text.
- [x] Workspace tests, strict Clippy, formatting, and `git diff --check` pass
      before feature review.

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

- **Relaxed evidence boundary:** exact versions, declaration paths, codec
  preservation, and negative probe sentinels are locked. Deterministic runtime
  probes are intentionally absent; declaration-managed status remains
  effective-unverified rather than being promoted to healthy.
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

## Pre-mortem and realized review handoff

The earlier pre-mortem assumed every admitted target needed a deterministic
runtime probe. The durable relaxed amendment supersedes that assumption: these
three targets are declaration-managed and expose no effective probe. The actual
residual risk is truthful declared/effective separation, not hidden runtime
activation.

- Kimi project MCP remains `Unsupported`; its global MCP is `Unverified` and
  only static, representable declarations are admitted.
- Vibe OAuth and SSE are unsupported; project trust and effective load remain
  unverified without approval or TUI/LLM execution.
- Kilo rejects unknown schema keys and conflicting document locations while
  preserving valid JSONC comments and unrelated fields.
- Foreground `--yes` acknowledges exact declaration/effective consequences;
  daemon cycles leave declaration-managed work pending. Removal retracts only
  proven skilltap-owned declarations and does not require effective evidence.

## Completion review handoff — 2026-07-15

All seven child checkpoints are complete under the relaxed amendment. The
implementation is committed in the contract-lock checkpoint plus the cohesive
adapter/acceptance checkpoint. The parent feature advances to `review`; an
independent review should verify the final diff and the residual unsupported
surfaces above rather than reintroducing the superseded probe requirement.
