---
id: epic-expanded-harness-support-trust-interactive
kind: feature
stage: implementing
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-project-skill-links, epic-expanded-harness-support-declaration-managed]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/junie-skills.md
  - .research/attestation/junie-mcp.md
  - .research/attestation/junie-extensions.md
  - .research/attestation/amp-manual.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-15
---

# Trust- and Interactive-State Adapters for Junie and Amp

## Brief

Deliver complete adapters for Junie and Amp while preserving their native trust,
interactive-state, skill-local MCP, and runtime-health semantics. Both targets
provide documented global and project skill and MCP surfaces, but configured
state is not always proof of effective availability; the adapters must expose
that distinction through normalized observation and actionable health.

The adapters consume the shared managed lifecycle, preserve unrelated native
configuration, and keep native extensions or skill-local MCP representations
only when they are the faithful form. Each target ships with isolated native
validation, trust-state cases, and the common adapter acceptance evidence.

## Epic context

- Parent epic: `epic-expanded-harness-support`.
- Position in epic: parallel concrete-adapter feature after the registry,
  managed fallback, and project-skill projection foundations.
- Implementation contract dependency: the target stories consume
  `epic-expanded-harness-support-file-managed-contracts`, which is the in-flight
  owner of scope-generic managed projection, registry-owned default binaries,
  source-only marketplace registration, bounded effective-state probes, and
  profile-driven fixture layouts. This feature does not create competing ports.

## Simplification opportunity

Reuse normalized declared-versus-effective observations, the shared managed
projection executor, and the completed project-skill link planner. Junie and Amp
should contain only target-owned paths, codecs, profile/probe decoders, and
faithfulness rules—not target-specific ownership, rollback, trust mutation, or
interactive-session automation.

## Foundation references

- `docs/VISION.md` — Observable Ownership, Native First, and Deep Support Over
  Broad Claims.
- `docs/SPEC.md` — target/scope selection, managed projection, ephemeral fresh
  observations, ownership, and unknown-version observe-only behavior.
- `docs/ARCH.md` — adapter ports, capability detection, declared/effective
  observation, bounded native processes, and revalidated execution.
- `docs/HARNESS-CONTRACTS.md` — Expanded Target Set, Scope Mapping, MCP Mapping,
  Unknown Harness Versions, and Adding Another Harness.
- `.research/analysis/briefs/current-agent-extension-standards.md` — separate
  desired, native-declared, and effective state; caches are observation evidence,
  never installation APIs.

## Grounding summary

The completed prerequisites supply the right ownership boundaries:

- `TargetRegistry` is the only production target list. Adapters provide identity,
  detection/profile selection, bounded observation, native roots,
  `SkillProjectionPort`, and optional lifecycle ports. CLI target validation,
  help, enablement, composition, and `--target all` derive from it.
- `ManagedProjectionPort::plan` receives one caller-resolved checkout for apply
  and no checkout for removal, then returns exact file/tree writes, a target-local
  projection manifest, and current/desired fingerprints. Shared orchestration
  owns source acquisition, drift, acknowledgment, pending recovery, rollback,
  and state journaling.
- Project standalone skills have one canonical
  `<project>/.agents/skills/<name>` tree. `SkillProjectionPort::destination`
  yields either a canonical no-op or a project-relative per-skill link whose
  ownership and inode are revalidated under lock.
- The in-flight file-managed contract story closes gaps that otherwise block
  Junie/Amp: the current managed path is project-only, is selected before exact
  profile authority is checked, and cannot run a bounded native effective-state
  probe. This feature depends on that shared correction rather than duplicating
  it.
- Current status maps coarse filesystem labels to `ObservationLayer::Effective`.
  That is not valid for Junie MCP state or Amp workspace MCP before runtime/trust
  evidence. The shared probe contract must preserve declared file observations
  even when effective state is unavailable, untrusted, auth-required, or failed.
- `ObservationLayer::{Declared, Effective}`, `TrustRequired`, `ConsentRequired`,
  `CapabilityUnverified`, and adoption's `DeclaredOnly` result already model the
  important semantics. No target-specific persisted trust/cache/session state is
  needed.

The source-direct native boundaries are:

| Target | Complete skills | Declared MCP | Effective/trust evidence |
|---|---|---|---|
| Junie | `~/.junie/skills/<name>` and `<project>/.junie/skills/<name>` | `~/.junie/mcp/mcp.json` and `<project>/.junie/mcp/mcp.json` | `/mcp` exposes scoped starting/active/inactive/disabled/failed/auth-required state, but the attested surface is interactive |
| Amp | supported user Agent/Amp roots and project `.agents/skills` | user settings and nearest project `.amp/settings.json` under `amp.mcpServers`; a skill may own local `mcp.json` | `amp mcp doctor` plus workspace trust; skill-local MCP retains relative-path and lazy-load semantics |

Junie's native extension manager is also interactive: project/user declarations
and cache content may be observed, but install/update/remove is documented only
through `/extensions`. The cache is never mutation authority.

The research does **not** attest exact Junie/Amp version output bytes, exact
known mutable versions, Amp's selected user settings path/precedence, or a
machine-readable Junie `/mcp` equivalent. Those are real contract blockers, not
implementation details. The first child locks them before either adapter enters
the canonical registry.

Mapping used direct repository reading only. Nested agents and peer mechanisms
were explicitly prohibited. The four child stories are durable checkpoints for
one cohesive Sol xhigh feature owner, not default parallel assignments.

## Design decisions

- **Consume the shared file-managed contracts.** Depend on
  `epic-expanded-harness-support-file-managed-contracts` for exact-scope managed
  projection, source-only marketplace registration, default-binary metadata,
  version-gated mutation, bounded effective probing, and profile-carried test
  layouts. This feature adds no second `ManagedActivationProbe`, runtime-status
  port, global managed orchestrator, or fixture switch.
- **Exact profile authority is evidence-produced.** No version literal, version
  argv, probe output grammar, or Amp user settings path is guessed. The contract
  lock captures one installed release per target in isolated roots and pins exact
  fixtures. Unknown/adjacent versions remain observe-only. A target that cannot
  close the minimum contract stays out of `TargetRegistry::canonical()`.
- **Minimum effective-observation gate is binding.** Whole-directory skill
  loading may be verified from the documented loader root plus isolated native
  acceptance. MCP configuration is emitted as `Declared`; an `Effective` MCP
  observation requires a deterministic bounded probe. If Junie has no
  non-interactive probe beyond `/mcp`, its MCP mutation capability remains
  unverified and the Junie adapter cannot be called complete. We do not drive a
  TTY, parse a cache as runtime truth, or relabel file presence as effective.
- **Trust and authentication are health, not drift or authority.** Amp's
  untrusted workspace and both targets' auth-required state leave declared bytes
  observable and owned while effective health is attention-required. Neither
  `--yes` nor a runtime probe grants mutation capability; only an exact compiled
  profile does. skilltap never edits trust decisions or OAuth/secret state.
- **No deterministic Junie extension mutation claim.** Observe native extension
  declarations as `Declared` plugin resources and preserve caches as read-only
  evidence. Do not expose native extension install/update/remove capabilities,
  write `extensions.json` as a lifecycle API, or mutate `~/.junie/extensions/`.
  Managed portable skill/MCP components use skilltap-owned projection instead.
- **Project standalone skills reuse the completed link service.** Junie's
  project destination is `<project>/.junie/skills`, so the shared planner derives
  a relative per-skill link to canonical `.agents/skills`. Amp's project
  destination is `.agents/skills`, so it produces `NotRequired` with no duplicate
  link/tree. Adapter code supplies roots and compatibility evidence only.
- **Global skill choices stay documented and target-local.** Junie uses
  `~/.junie/skills`. Amp uses documented `~/.agents/skills` as the managed
  portable root and observes its other supported user/Amp roots for precedence
  or unmanaged conflicts; it does not duplicate a managed tree across every
  compatible root.
- **Managed plugins use one source-side normalizer.** Both ports consume the
  private selected-source/complete-tree helper delivered by the file-managed
  contract. They retain distinct destination codecs and compatibility rules.
  Required malformed/unsupported components block; optional unsupported
  components become exact `ManagedProjection::Omitted` evidence only after
  foreground acknowledgment.
- **Junie MCP is scoped JSON, not extension state.** Its codec edits only the
  fixture-locked MCP server container in the documented global/project
  `mcp.json`, preserving unrelated keys and unowned servers. Same-name unowned
  entries conflict. Secret values remain native; portable writes admit only
  references and supported transport/auth semantics.
- **Amp chooses representation by behavior.** Root/plugin MCP that is faithfully
  independent maps into scoped `amp.mcpServers`. A `mcp.json` owned by a complete
  skill remains inside that skill when relative paths or lazy activation are
  behavior-bearing. It is not duplicated into workspace settings. The skill
  tree fingerprint owns the bytes; `ManagedProjection::Mcp` records the exact
  server fingerprint for drift/removal evidence.
- **Interactive/cache observations stay ephemeral.** Fresh trust, effective MCP,
  native extension, and cache observations are not added to `state.json`.
  Successful declared writes retain ordinary provenance/apply evidence; status
  re-probes effective health each run.
- **No first-party bootstrap or instruction expansion.** Both targets are
  `DistributionSurface::Managed`, have no first-party bootstrap eligibility,
  and advertise instruction support only if a separately attested native
  contract exists. This feature does not infer instruction behavior from skill
  loading.
- **No UI work.** This is a deterministic CLI/adapter capability; there is no
  screen or flow surface.
- **Review policy.** Effective review weight is `standard`: one independent
  feature-level pass after child verification, receiver adjudication, material
  blocker fixes, and verification, with no second independent pass. Design-time
  advisory review was skipped because this invocation forbids nested agents and
  peers; design review is non-blocking.

## Architectural choice

**Chosen — two distinct adapters over the shared scope/probe/source toolkit.**
`JunieAdapter` and `AmpAdapter` remain separate registry entries. The shared
contracts own exact-scope lifecycle, bounded execution, normalized
observations, source extraction, target-local ownership, and project links.
Each adapter owns only its exact profile, paths, MCP document codec, effective
probe decoder, precedence, and compatibility semantics.

**Rejected — a generic `TrustInteractiveAdapter` descriptor.** Junie's
interactive extension/cache plane and scoped `/mcp` state do not match Amp's
workspace trust, doctor command, settings precedence, or skill-local lazy MCP.
Turning these into flags would hide native contracts behind a universal format.

**Rejected — automate slash/TUI flows or infer from caches.** A pseudo-terminal
would violate deterministic non-interactive operation, and cache presence is
not a supported installation or effective-load API. Both approaches would grant
mutation/health claims from evidence that the product explicitly treats as
non-authoritative.

## Implementation Units

### Unit 1: Lock exact Junie and Amp native contracts

**Files**:

- `crates/harnesses/src/adapters/trust_interactive/contracts.rs` (new).
- `crates/harnesses/tests/fixtures/trust_interactive/{junie,amp}/` (new bounded,
  non-secret version/config/probe fixtures).
- focused contract tests under
  `crates/harnesses/src/adapters/trust_interactive/`.

**Story**: `epic-expanded-harness-support-trust-interactive-contract-lock`

```rust
pub(super) struct VerifiedTrustInteractiveContract {
    pub verified_version: &'static str,
    pub profile_id: &'static str,
    pub default_binary: &'static str,
    pub version_arguments: &'static [&'static str],
    pub mcp: VerifiedMcpContract,
}

pub(super) struct VerifiedMcpContract {
    pub global_document: &'static str,
    pub project_document: &'static str,
    pub effective_probe: EffectiveProbeContract,
}

pub(super) enum EffectiveProbeContract {
    BoundedProcess {
        arguments: &'static [&'static str],
        output: EffectiveOutputContract,
    },
    InteractiveOnly,
}
```

**Implementation notes**:

- Populate constants only from refreshed source-direct evidence and isolated
  installed-binary capture; the design deliberately supplies no placeholders.
- Pin exact version bytes, supported config shape/precedence, trust/auth/runtime
  states, and probe argv/output. Fixture payloads contain inert references only.
- For Junie, explicitly test whether a deterministic non-TTY status command/API
  exists. `InteractiveOnly` is a blocker for MCP mutation admission, not a
  license to parse cache state or script `/mcp`.
- For Amp, pin the user settings path and precedence against nearest project
  `.amp/settings.json`, plus `mcp doctor`'s exact bounded output contract.
- Verify Amp skill-local `mcp.json` relative-path and lazy-load behavior rather
  than assuming scoped settings are equivalent.

**Acceptance criteria**:

- [ ] Exact known version bytes select one verified profile per target; malformed,
      extra-document, control-character, adjacent, and unknown versions cannot
      grant mutation authority.
- [ ] Global/project skill roots, MCP document paths/keys, precedence, and
      unknown-field preservation are fixture-locked.
- [ ] Effective probes distinguish loaded, disabled/inactive, trust-required,
      authentication-required, failed, and unverified state without raw output
      entering domain findings.
- [ ] A target lacking deterministic effective MCP observation remains explicitly
      blocked and unregistered; independently closed sibling evidence is retained.

---

### Unit 2: Junie adapter and scoped managed projection

**Files**:

- `crates/harnesses/src/adapters/trust_interactive/mod.rs` (new).
- `crates/harnesses/src/adapters/trust_interactive/junie.rs` (new).
- `crates/harnesses/src/adapters/trust_interactive/junie_projection.rs` (new).
- `crates/harnesses/src/adapters/mod.rs` and `crates/harnesses/src/lib.rs`
  (exports only after the contract is locked).

**Story**: `epic-expanded-harness-support-trust-interactive-junie`

```rust
pub struct JunieAdapter;
pub struct JunieSkillProjection;
pub struct JunieManagedProjection;
pub struct JunieEffectiveStateProbe;

impl HarnessAdapter for JunieAdapter {
    fn identity(&self) -> TargetIdentity;
    fn version_arguments(&self) -> Vec<OsString>;
    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, DetectionError>;
    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection;
    fn observe(
        &self,
        paths: &PlatformPaths,
        scope: &Scope,
        limits: ExternalTreeLimits,
    ) -> Result<AdapterObservationPaths, ObservationPathError>;
    fn skill_projection(&self) -> Option<&dyn SkillProjectionPort>;
    fn managed_projection(&self) -> Option<&dyn ManagedProjectionPort>;
    fn effective_state_probe(&self) -> Option<&dyn EffectiveStateProbePort>;
}

struct JunieMcpDocument {
    root: serde_json::Map<String, serde_json::Value>,
}

impl JunieMcpDocument {
    fn parse(bytes: Option<&[u8]>, limits: JsonLimits) -> Result<Self, JunieCodecError>;
    fn upsert(&mut self, id: &NativeId, server: &PortableMcpServer)
        -> Result<(), JunieCodecError>;
    fn remove(&mut self, id: &NativeId) -> Result<(), JunieCodecError>;
    fn managed_fingerprint(&self, ids: &BTreeSet<NativeId>) -> Option<Fingerprint>;
    fn encode(self) -> Result<Option<Vec<u8>>, JunieCodecError>;
}
```

**Implementation notes**:

- Identity is `junie` / `Junie`, managed distribution, with default binary and
  exact profile from Unit 1. No native lifecycle port is exposed.
- `SkillProjectionPort` returns `~/.junie/skills` globally and
  `<project>/.junie/skills` in projects. Project standalone install/status uses
  the shared relative-link contract; no adapter symlink code is added.
- Managed plugin skill trees use the shared source normalizer and scoped
  execution. The adapter writes only documented skill/MCP load surfaces and
  never the extension cache.
- The MCP codec edits only owned server members in
  `~/.junie/mcp/mcp.json` or `<project>/.junie/mcp/mcp.json`, preserving unknown
  fields and unowned siblings. The exact server container/transport grammar
  comes from Unit 1.
- Native extension declarations are read-only `Declared` plugin evidence.
  Interactive `/extensions` state and cached content never grant install,
  update, remove, or healthy-effective claims.
- When Unit 1 locks a bounded MCP probe, its decoder emits effective resources
  and registered health. Otherwise this story remains blocked; it does not
  substitute file presence.

**Acceptance criteria**:

- [ ] Known profile supports documented skill/MCP observation and managed
      projection at both scopes; unknown versions plan no writes.
- [ ] A project standalone skill has one canonical tree and one correct owned
      relative link under `.junie/skills`; repair/removal reuse existing
      no-follow behavior.
- [ ] Scoped MCP install/update/remove preserves unknown fields and unowned
      servers; drift and same-name conflicts block rather than overwrite.
- [ ] Declared and effective MCP states remain distinct; disabled, failed, or
      auth-required state is attention-required and not drift.
- [ ] Native extension declarations/caches are unchanged byte-for-byte and no
      interactive lifecycle capability is advertised.
- [ ] Immediate repeats produce no file, link, operation, or target-state change.

---

### Unit 3: Amp adapter, trust, and skill-local MCP

**Files**:

- `crates/harnesses/src/adapters/trust_interactive/amp.rs` (new).
- `crates/harnesses/src/adapters/trust_interactive/amp_projection.rs` (new).
- adapter module/public exports only after the contract is locked.

**Story**: `epic-expanded-harness-support-trust-interactive-amp`

```rust
pub struct AmpAdapter;
pub struct AmpSkillProjection;
pub struct AmpManagedProjection;
pub struct AmpEffectiveStateProbe;

#[derive(Clone, Debug, Eq, PartialEq)]
enum AmpMcpPlacement {
    ScopedSettings,
    SkillLocal { skill: AgentSkillName },
}

struct AmpSettingsDocument {
    root: serde_json::Map<String, serde_json::Value>,
}

impl AmpSettingsDocument {
    fn parse(bytes: Option<&[u8]>, limits: JsonLimits) -> Result<Self, AmpCodecError>;
    fn upsert(&mut self, id: &NativeId, server: &PortableMcpServer)
        -> Result<(), AmpCodecError>;
    fn remove(&mut self, id: &NativeId) -> Result<(), AmpCodecError>;
    fn managed_fingerprint(&self, ids: &BTreeSet<NativeId>) -> Option<Fingerprint>;
    fn encode(self) -> Result<Option<Vec<u8>>, AmpCodecError>;
}
```

**Implementation notes**:

- Identity is `amp` / `Amp`, managed distribution, with exact default binary and
  profile from Unit 1. No native marketplace/plugin lifecycle is claimed.
- Project standalone skills consume canonical `<project>/.agents/skills`
  directly (`projection=not_required`). Global managed skills use documented
  `~/.agents/skills`; other Amp/user roots are observed for precedence and
  unmanaged collisions, never synchronized as duplicate trees.
- Scoped MCP edits only `amp.mcpServers` in Unit 1's locked user settings file
  or nearest `<project>/.amp/settings.json`. Unknown settings and unowned
  servers survive. Literal secret/OAuth state remains outside skilltap.
- A skill-owned `mcp.json` remains within the complete skill tree when relative
  paths or lazy loading are required. The adapter records its per-server
  fingerprint in the projection manifest but does not duplicate it into scoped
  settings. Root-level independent MCP declarations use scoped settings.
- `amp mcp doctor` runs through the shared bounded probe port in the selected
  project working directory. Trusted healthy servers become `Effective`;
  untrusted workspace declarations remain `Declared` with `trust.required`.
- Trust approval, auth flows, and doctor output are observation only. They never
  enter inventory/state as policy and never widen a compiled capability.

**Acceptance criteria**:

- [ ] Known profile supports both scopes; unknown/changed output remains
      observe-only and cannot reach managed execution.
- [ ] Project canonical skills produce no redundant link; global skills occupy
      only the selected documented portable root and preserve other roots.
- [ ] User/workspace settings precedence and `amp.mcpServers` merge/remove
      preserve unknown and unmanaged content.
- [ ] Untrusted workspace status reports declared configuration plus
      `trust.required`, never effective/healthy or drift; trusted doctor evidence
      produces exact effective health.
- [ ] Skill-local MCP retains relative paths and lazy activation without a
      duplicate settings entry; update/removal fingerprints the owned skill and
      MCP identities safely.
- [ ] Immediate repeat, target isolation, pending recovery, rollback, and
      required/optional compatibility behavior pass through shared machinery.

---

### Unit 4: Integrated registry and acceptance evidence

**Files**:

- `crates/harnesses/src/adapters/mod.rs`, `crates/harnesses/src/lib.rs`, and
  `crates/harnesses/src/registry.rs` — canonical registration after both target
  contracts close.
- `crates/harnesses/tests/detection.rs` and target adapter contract tests.
- `crates/test-support/src/harness_profile.rs`,
  `crates/test-support/src/managed_acceptance.rs`, and isolated fixture helpers.
- `crates/cli/src/application/tests.rs` and
  `crates/cli/tests/compiled_binary.rs` — lifecycle/status/output acceptance.

**Story**: `epic-expanded-harness-support-trust-interactive-acceptance`

**Implementation notes**:

- Insert `junie` and `amp` into the canonical registry's product order without a
  second CLI/test target list. First-party bootstrap remains Codex/Claude only.
- Add profile-carried Junie/Amp layouts and effective-state responses; do not add
  id matches to `FakeHarnessProfile::layout`.
- Run the shared adapter and managed-projection matrices for both targets. The
  production-aware callback performs real assertions before returning evidence
  labels.
- Exercise declared/effective divergence, Junie interactive/native-extension
  preservation, Amp trust, and both project-skill projection shapes in compiled
  CLI tests.

**Acceptance criteria**:

- [ ] Registry-derived help, enable/list, config policy, JSON, and `--target all`
      include Junie/Amp; bootstrap excludes them.
- [ ] Exact known profiles are mutable only for locked capabilities/scopes;
      unknown versions and probe mismatches remain observe-only.
- [ ] Junie passes relative project-link acceptance and Amp passes canonical
      no-link acceptance with complete skill siblings/executable intent.
- [ ] Both targets pass global/project managed source registration,
      skill+MCP install/update/remove, unknown-field preservation, drift,
      acknowledgment, target-local state, recovery, rollback, and repeat
      idempotency.
- [ ] Plain and JSON status derive from one typed outcome and distinguish
      declared, effective, trust/auth/interactive-unverified, drift, and
      conflicts without raw payloads.
- [ ] Junie extension caches/state and Amp trust/auth state remain unmodified.
- [ ] Workspace tests, all-feature Clippy with warnings denied, formatting, and
      `git diff --check` pass before standard feature review.

## Implementation Order

1. `epic-expanded-harness-support-trust-interactive-contract-lock` — Unit 1,
   `depends_on: [epic-expanded-harness-support-file-managed-contracts]`.
2. `epic-expanded-harness-support-trust-interactive-junie` — Unit 2,
   `depends_on: [epic-expanded-harness-support-trust-interactive-contract-lock]`.
3. `epic-expanded-harness-support-trust-interactive-amp` — Unit 3,
   `depends_on: [epic-expanded-harness-support-trust-interactive-contract-lock]`.
4. `epic-expanded-harness-support-trust-interactive-acceptance` — Unit 4,
   `depends_on: [epic-expanded-harness-support-trust-interactive-junie,
   epic-expanded-harness-support-trust-interactive-amp]`.

The target stories are dependency-parallel after the contract lock, but the
normal execution is one Sol xhigh feature owner carrying both checkpoints
sequentially (Junie, then Amp) to keep registry/source/test context cohesive.
The acceptance checkpoint waits for both. `.work/bin/work-view --blocking` was
run for every child receiving a dependency before these edges were written; no
existing dependents or cycles were reported.

## Simplification

- Consume the file-managed contract story instead of creating another
  scope/probe/default-binary/source abstraction.
- Keep trust and interactive/cache/session state ephemeral; do not add it to
  `TargetResourceState` or introduce target-specific reconciliation states.
- Use `ObservationLayer::{Declared, Effective}` and existing registered trust,
  consent, and capability findings rather than a Junie/Amp status enum.
- Reuse the project-skill service: Junie supplies a distinct root and gets a
  link; Amp supplies the canonical root and gets no operation.
- Reuse shared selected-source parsing, ownership, drift, acknowledgment,
  rollback, pending recovery, and acceptance; do not copy Codex projection
  orchestration.
- Preserve Amp skill-local MCP inside the complete tree rather than extracting
  and duplicating it into settings.
- Do not add native extension/plugin lifecycle wrappers, pseudo-TTY automation,
  cache mutation, trust mutation, OAuth storage, or first-party bootstrap.

No cleanup child is warranted. Each removal/consolidation is coupled to the
contract or adapter checkpoint that proves its replacement.

## Testing

- **Contract fixtures:** exact version bytes, exact native documents, precedence,
  effective status states, and interactive-only failure. Protects mutation
  authority from guessed versions/parsers.
- **Codec tests:** unknown/unowned preservation, same-name conflicts, managed
  entry fingerprints, malformed/duplicate documents, portable references, and
  remove-to-empty behavior. Protects user-authored native config.
- **Observation tests:** declared/effective pairing, declared-only adoption
  rejection, trust/auth/failed health, unknown-profile downgrade, and no raw
  payload fields. Protects honest status.
- **Project-skill tests:** Junie relative link and Amp canonical no-op, including
  nested projects, repair/removal, complete siblings, and no-follow safety.
- **Managed acceptance:** both scopes, source-only marketplace state, complete
  trees, MCP representation, omissions/required blockers, ownership/drift,
  target isolation, pending recovery, rollback, and immediate repeat.
- **Compiled CLI tests:** registry-derived target exposure, bootstrap exclusion,
  plain/JSON parity, next actions, and byte-for-byte preservation of native
  caches/trust/auth state.
- **Test economy:** pin stable ids, paths, schemas, findings, operation surfaces,
  and results—not incidental help snapshots, parser branches, or getters.

## Risks and contract blockers

- **Junie effective MCP may be interactive-only.** This is the largest blocker.
  If Unit 1 cannot establish a deterministic bounded non-TTY observation
  surface, Junie cannot meet the minimum effective-state contract. The fallback
  is an explicitly observe-only/unregistered target, not cache inference,
  pseudo-TTY automation, or file-presence-as-effective.
- **Exact native versions are absent from research.** Neither adapter receives a
  mutable profile until isolated validation pins exact output. Unavailable
  binaries or unstable decoders block that target independently.
- **Amp user path/precedence is under-attested.** The project file and settings
  key are known, but the selected user settings path and precedence among
  compatible roots require Unit 1 evidence. The adapter must not write a guessed
  XDG/home file.
- **Amp skill-local MCP can be accidentally de-lazified.** Moving it to workspace
  settings could change relative path resolution and startup behavior. The
  placement decision is source-shape and semantics aware; ambiguous mappings are
  partial/blocked.
- **Shared contract is in flight.** The target contract lock is explicitly
  blocked by `epic-expanded-harness-support-file-managed-contracts`. If that
  implementation diverges from its approved scope/probe/source contract, amend
  the shared owner first; do not fork a Junie/Amp-only implementation.
- **Declared success versus effective attention.** Correct owned bytes may be
  installed while trust/auth/reload state remains unresolved. Repeats must stay
  no-op while status remains attention-required; neither destructive rollback
  nor repeated rewrites resolves an interactive decision.

## Pre-mortem

- **Riskiest assumption:** Junie exposes a deterministic effective MCP observer
  despite the current attestation documenting only `/mcp`.
- **Production failure condition:** skilltap writes valid config, then reports
  healthy even though a trust/auth/interactive/lazy-load boundary prevents the
  harness from loading it.
- **Mitigation:** exact-profile codecs, declared/effective layers, bounded fresh
  probes, and no cache/session authority.
- **Fallback:** retain honest observation diagnostics and do not register the
  affected managed mutation capability or complete the target.
- **Least certain area:** Junie's non-interactive runtime surface and Amp's exact
  user settings/root precedence. Unit 1 isolates both before adapter mutation.

## Design review note

Design-time advisory review was intentionally skipped because the caller
forbade nested agents and peer mechanisms. The implementation remains subject
to the requested standard feature review after all child checkpoints verify.
