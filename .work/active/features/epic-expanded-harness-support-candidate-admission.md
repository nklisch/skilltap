---
id: epic-expanded-harness-support-candidate-admission
kind: feature
stage: review
tags: []
parent: epic-expanded-harness-support
depends_on: [epic-expanded-harness-support-registry, feature-managed-fallback-target-parity, epic-expanded-harness-support-project-skill-links]
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-targets-skills-mcp-2026-07-12.md
  - .research/attestation/cursor-skills.md
  - .research/attestation/cursor-mcp.md
  - .research/attestation/zoocode-skills.md
  - .research/attestation/zoocode-mcp.md
  - .research/attestation/zcode-skills.md
  - .research/attestation/zcode-mcp.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-12
updated: 2026-07-14
---

# Cursor, Zoo Code, and ZCode Candidate Admission

## Brief

Run independent admission tracks for Cursor, Zoo Code, and ZCode without
weakening mutation authority. Each begins as an observe-only candidate until
isolated boundary validation attests its exact supported global and project
write files, complete-skill sibling behavior, MCP reload and precedence, and
cache-independent effective-state observation.

When one target clears the gate, it consumes the shared registry and managed
adapter contract and receives its own verified mutable profile and acceptance
evidence. One successful candidate never grants authority to the other two, and
an unresolved platform path or editor-internal dependency remains an explicit
block rather than a guessed configuration location.

## Implementation result

All three independent tracks resolved honestly to `blocked`: Cursor lacks an
installed exact runtime and redirectable editor/CLI profile; Zoo Code lacks a
standalone process plus documented cross-platform global MCP and non-UI
effective observation; ZCode lacks deterministic installation identity, its
exact project skill path, and a non-UI reload observer. The shared
`VerifiedObserveOnly` authority and candidate admission matrix landed without
registering any candidate. Aggregate acceptance proves Cursor, Zoo Code, and
ZCode remain absent from the registry, help, mutation, and `--target all`
surfaces, while Codex/Claude bootstrap behavior and sibling isolation remain
unchanged. Workspace verification passed 656 tests with strict Clippy,
formatting, and diff checks.

## Epic context

- Parent epic: `epic-expanded-harness-support`
- Position in epic: parallel boundary-validation and target-local admission
  after the registry and managed fallback foundations.

## Simplification opportunity

- Use the ordinary profile admission mechanism for individually verified
  candidates; do not create a second experimental-adapter execution path.

## Foundation references

- `docs/VISION.md` — Deep Support Over Broad Claims.
- `docs/ARCH.md` — Capability Detection, Observation, Mutation Safety.
- `docs/HARNESS-CONTRACTS.md` — Expanded Target Set, Adding Another Harness.

## Grounding summary

The exact source-direct candidate evidence establishes behavior but leaves a
different write-boundary gap for each target:

| Candidate | Established now | Missing before mutation admission |
|---|---|---|
| Cursor | Agent Skills in editor and CLI; skills can contain scripts/resources; MCP files at `~/.cursor/mcp.json` and `<project>/.cursor/mcp.json`; `cursor-agent mcp list` and per-server tool inspection | Exact documented global/project skill roots, complete-sibling discovery/update behavior, precedence/reload, and an exact version profile |
| Zoo Code | Complete `SKILL.md` directories with linked siblings under documented project/global `.roo` and `.agents` search families; project-over-global precedence; project `.roo/mcp.json`; global `mcp_settings.json`; stdio/HTTP/SSE and per-tool policy | Stable platform-independent global MCP pathname, deterministic extension/version detection, supported non-interactive reload/effective observation, and proof that no editor storage/cache write is required |
| ZCode | Global `~/.zcode/skills`; directory skills; copy/symlink imports for global or current project; user/workspace MCP behavior and editable enablement | Exact documented project skill destination, exact global/project MCP filenames, direct-edit/reload contract, cache-independent effective observation, and exact version/detection identity |

The attested gaps are admission blockers, not implementation placeholders. An
empirically discovered path is insufficient unless the current supported native
contract also documents that path as a write surface. A UI that can display a
setting is insufficient unless isolated validation can re-observe effective
state deterministically without writing editor or extension caches.

The completed prerequisites provide the reusable implementation seams:

- `TargetRegistry`, `HarnessAdapter`, `TargetIdentity`, exact-version
  `CapabilityProfileSelection`, registry-derived CLI validation/help, and
  adapter-owned optional ports live under `crates/harnesses/src/registry.rs`.
- `ManagedProjectionPort` returns target-native confined writes plus exact
  manifest/fingerprint evidence while shared orchestration owns source checkout,
  target-local ownership, drift, acknowledgment, rollback, and idempotency.
- `epic-expanded-harness-support-file-managed-contracts` is the in-flight
  checkpoint that makes this managed path global/project scoped, adds
  registry-owned default binary metadata, and adds bounded effective-state
  probes. Candidate admission stories depend on that checkpoint rather than
  duplicating its contract.
- Project standalone skills already use one validated canonical
  `<project>/.agents/skills/<name>` tree and derive a per-target relative link
  from `SkillProjectionPort::destination`. Candidate adapters provide only the
  verified native root and compatibility evidence; they never implement link
  ownership, repair, or removal.
- `FakeHarnessProfile`, `acceptance_matrix`, `ManagedProjectionProfile`, and
  `managed_acceptance_matrix` are the shared isolated fixture/production-aware
  acceptance surfaces. They currently cover Codex/Claude and managed fallback;
  this feature adds an admission gate in the same dependency-neutral style.
- Revalidated execution, root-confined no-follow filesystem access,
  target-local state updates, identity-aware rollback, bounded native process
  requests, and isolated fixture roots are established project patterns and are
  mandatory for any admitted candidate.

The current canonical registry contains only Codex and Claude. No candidate id,
path, binary, or mutable profile is present in production code, so there is no
legacy candidate behavior to preserve.

This design used direct reading only. The caller explicitly prohibited nested
agents and peer mechanisms. The child stories are durable validation and
admission checkpoints for one cohesive Sol xhigh feature owner, not eight
parallel implementation assignments.

## Design decisions

- **Admission is target-local and all-or-nothing for mutation.** Cursor, Zoo,
  and ZCode each receive an independent evidence report and disposition. One
  target's paths, version, or acceptance result can never populate another
  target's profile or unblock another target's admission story.
- **Three valid dispositions:**
  1. `admitted` — deterministic detection plus every skill, MCP, observation,
     ownership, and acceptance check passes; register the ordinary adapter and
     exact mutable profile.
  2. `observe_only` — exact version/detection and documented read-only surfaces
     are safe, but one or more mutation checks remain unresolved; register only
     a read-only adapter/profile, with no skill projection or managed projection
     port.
  3. `blocked` — deterministic installation identity or safe documented read
     surfaces are not established; do not register the target. Record the exact
     missing evidence in its boundary and admission story bodies.
  Both `observe_only` and `blocked` are complete, successful story outcomes;
  neither is softened into speculative support.
- **Represent a known observe-only profile accurately.** Extend
  `CapabilityProfileSelection` with `VerifiedObserveOnly { id, capabilities }`.
  It has `ProfileAuthority::ObserveOnly`, exposes observation capabilities and a
  profile id, returns no mutation capabilities, and remains observe-only after
  runtime narrowing. This avoids falsely labeling an exact validated candidate
  version as an unknown version and avoids misusing `VerifiedCompiled` merely
  because its mutation capability set is empty.
- **Source-direct documentation and isolated behavior are both required.** The
  gate accepts an exact path only when a current official source identifies it
  as supported and an isolated native instance proves write, reload,
  precedence, update, and removal behavior. Source-only claims lack effective
  evidence; empirical-only paths may be internal caches.
- **Validation never mutates operator state.** Candidate probes use isolated
  HOME/XDG/project/editor-profile roots and explicitly supplied binaries or
  extension hosts. A probe that cannot redirect every affected root is blocked;
  it never falls back to the operator's real editor profile.
- **No UI automation as a production adapter.** A deterministic official CLI or
  extension-host API may supply detection/effective observation. Screen
  scraping, settings-window automation, undocumented extension storage, and
  editor cache inspection cannot grant admission. They may reveal a research
  lead, but the candidate stays blocked until the supported boundary is
  documented and repeatable.
- **Use ordinary adapters after admission.** An admitted target implements its
  own `HarnessAdapter`, `SkillProjectionPort`, `ManagedProjectionPort`, native
  MCP codec, and `EffectiveStateProbePort`, then enters
  `TargetRegistry::canonical()`. There is no candidate executor, generic
  editor adapter, runtime path override, or second registry.
- **A read-only candidate has no latent mutator.** An `observe_only` adapter may
  implement version decode, profile selection, bounded observation, native root,
  and a read-only effective probe. It returns `None` for `skill_projection()` and
  `managed_projection()`, reports no native lifecycle, and never supplies a
  `VerifiedCompiled` profile. Promotion adds ports and authority together in the
  same target admission checkpoint.
- **Managed projection stays shared.** Admitted candidates consume the exact
  global/project `ManagedProjectionContext` and shared file-managed source
  reader delivered by `epic-expanded-harness-support-file-managed-contracts`.
  Candidate modules own only their native skill roots, MCP codec, documented
  precedence, and effective decoder. They do not duplicate acquisition,
  ownership, state refresh, rollback, or partial-component policy.
- **Project skills stay canonical.** The adapter returns its verified project
  skill root. If that root is `<project>/.agents/skills`, the existing planner
  emits no link. Otherwise it derives one per-skill relative link. A candidate
  cannot be admitted if a complete directory reached through that representation
  loses siblings, executable intent, or target precedence.
- **No partial scope admission.** A candidate with a validated project surface
  but unresolved global surface, or vice versa, remains observe-only/blocked.
  The product contract requires both scopes; a global-only mutable profile is
  not shipped as an intermediate result.
- **No native lifecycle claims.** Current evidence establishes component load
  surfaces, not complete marketplace/plugin lifecycle. Candidate adapters use
  source-only marketplace registration plus managed projection and return no
  native lifecycle unless a separate future source-direct contract establishes
  one.
- **Secrets and editor state remain outside skilltap.** OAuth tokens, extension
  credentials, user trust decisions, editor databases, and caches are neither
  copied nor treated as desired state. MCP codecs preserve references and reject
  literal secret acquisition under the existing compatibility rules.
- **Feature closure does not require three admissions.** The feature is complete
  when every target has a grounded disposition and the aggregate tests prove
  admitted/observe-only/blocked isolation. A blocked candidate is an honest
  product result, not a reason to guess or keep the feature indefinitely open.
- **No skilltap UI surface.** This is a non-interactive CLI/adapter boundary.
  External editor UI may be part of disconfirming validation, but skilltap gains
  no screen or flow, so mockup fallback is skipped.
- **Review posture.** Effective review weight is `standard`: after all child
  checkpoints resolve and integrated verification is green, run exactly one
  independent feature-level pass, adjudicate findings, fix material blockers,
  and verify without a second independent pass. Design-time advisory review is
  skipped because the caller prohibited nested agents and peer mechanisms.

## Architectural choice

**Chosen — evidence-gated ordinary adapters with a test-only admission matrix.**
A dependency-neutral candidate matrix names the exact proof required for one
target and produces `admitted`, `observe_only`, or `blocked`. Each target first
runs a boundary story and records source/version/path/reload evidence in that
item. Its admission story then either implements an ordinary adapter using the
shared registry/projection/link contracts, implements a strictly read-only
adapter, or proves the target remains absent. A final acceptance story checks
that the three results remain isolated.

**Rejected — add three mutable skeleton adapters and fill paths later.** A
skeleton with guessed roots or empty effective decoders would place mutation
ports in production before their boundary is established. Runtime probes cannot
grant missing authority, so such a skeleton has no safe upgrade path.

**Rejected — one descriptor-driven editor adapter.** Cursor is CLI/editor
hybrid, Zoo is an editor extension with `.roo`/`.agents` precedence, and ZCode
has import-managed skill/MCP state. Flattening those into path/schema fields
would obscure detection identity, reload, precedence, and ownership differences
and recreate a universal plugin format.

**Rejected — treat successful file writes as admission.** A write that appears
on disk does not prove the harness loaded it, selected the expected scope, or
will preserve/removal-update it safely. Admission requires effective
observation and immediate-repeat acceptance.

## Implementation Units

### Unit 1: Candidate admission authority and shared gate

**Files**:

- `crates/core/src/domain/installation.rs` — accurate verified-observe-only
  profile variant and invariants.
- `crates/test-support/src/candidate_admission.rs` (new) — dependency-neutral
  gate scenarios, checks, dispositions, and report.
- `crates/test-support/src/lib.rs` — gate re-exports.
- Narrow profile/render tests in `crates/core/src/domain/installation.rs` and
  `crates/cli/src/application/tests.rs` only where the new authority result is
  rendered.

**Story**: `epic-expanded-harness-support-candidate-admission-gate`

```rust
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "authority", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapabilityProfileSelection {
    VerifiedCompiled {
        id: CapabilityProfileId,
        capabilities: ScopedCapabilitySets,
    },
    VerifiedObserveOnly {
        id: CapabilityProfileId,
        capabilities: ScopedCapabilitySets,
    },
    UnknownVersion {
        capabilities: ScopedCapabilitySets,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CandidateAdmissionCheck {
    ExactInstallationIdentity,
    DocumentedGlobalSkillRoot,
    DocumentedProjectSkillRoot,
    CompleteSkillSiblings,
    SkillPrecedenceAndReload,
    DocumentedGlobalMcpFile,
    DocumentedProjectMcpFile,
    McpSchemaAndPrecedence,
    EffectiveReloadObservation,
    UnknownFieldAndSiblingPreservation,
    OwnershipSafeUpdateAndRemoval,
    CacheIndependentBoundary,
    SharedAdapterAcceptance,
    ImmediateRepeatNoChange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateDisposition {
    Admitted,
    ObserveOnly,
    Blocked,
}

pub struct CandidateAdmissionEvidence {
    checks: BTreeSet<CandidateAdmissionCheck>,
}

pub struct CandidateAdmissionReport {
    candidate: &'static str,
    disposition: CandidateDisposition,
    checks: BTreeSet<CandidateAdmissionCheck>,
    missing: Vec<CandidateAdmissionCheck>,
}

pub fn candidate_admission_gate(
    candidate: &'static str,
    exercise: impl FnMut(CandidateAdmissionCheck) -> bool,
) -> CandidateAdmissionReport;
```

**Implementation notes**:

- `VerifiedObserveOnly` returns `ProfileAuthority::ObserveOnly`, returns its id
  from `profile_id()`, returns its scoped capabilities from
  `observation_capabilities()`, and returns `None` from
  `mutation_capabilities()`. `narrow()` preserves the variant.
- The matrix is test support, not runtime authority. `Admitted` requires every
  check. `ObserveOnly` requires exact installation identity plus documented,
  deterministic read-only observation but is missing one or more mutation
  checks. If those read prerequisites fail, disposition is `Blocked`.
- The production-aware target runners perform concrete assertions before
  returning a check, following the existing managed acceptance pattern; labels
  alone are not evidence.
- Exact source URLs, fetched dates, version bytes, paths, commands, and observed
  outcomes live in each boundary story body. No new research or progress file
  is created.

**Acceptance criteria**:

- [ ] A verified observe-only profile has a stable id and observation
      capabilities but cannot expose mutation capabilities under construction
      or narrowing.
- [ ] Existing verified-compiled and unknown-version serialization and behavior
      remain unchanged.
- [ ] The gate cannot produce `Admitted` while any required check is absent.
- [ ] The gate cannot produce `ObserveOnly` without deterministic exact
      installation identity and safe documented observation.
- [ ] No gate type or disposition is consulted by the production executor;
      ordinary profile/port contracts remain the only runtime authority.

---

### Unit 2: Zoo Code boundary validation

**Files**:

- Story evidence in
  `.work/active/stories/epic-expanded-harness-support-candidate-admission-zoo-boundary.md`.
- `crates/harnesses/tests/candidate_zoo_boundary.rs` only if an official,
  redirectable, deterministic extension-host/CLI validation boundary is found.
- `crates/test-support/src/harness_profile.rs` only after exact version bytes and
  isolated layout are proven.

**Story**: `epic-expanded-harness-support-candidate-admission-zoo-boundary`

Validation starts with the riskiest candidate. It must identify the exact Zoo
extension identity and version observation mechanism; enumerate every supported
user/project skill root and precedence; resolve the stable global
`mcp_settings.json` path for every supported platform; retain project
`.roo/mcp.json`; and prove restart/reload plus effective server/tool observation
without editing VS Code-compatible internal storage or caches.

**Acceptance criteria**:

- [ ] The story body records one source-bound and isolated result for every gate
      check, including disconfirming attempts and exact missing evidence.
- [ ] Every global/project root and file is official, platform-resolved, and
      redirectable into an isolated profile; no host editor profile is touched.
- [ ] Complete skill siblings, project/global collision precedence, MCP
      transports/policy, unknown-field preservation, update, removal, and repeat
      behavior are observed from the effective harness state.
- [ ] The story concludes `admitted`, `observe_only`, or `blocked`; a UI-only or
      cache-dependent result cannot conclude `admitted`.

---

### Unit 3: ZCode boundary validation

**Files**:

- Story evidence in
  `.work/active/stories/epic-expanded-harness-support-candidate-admission-zcode-boundary.md`.
- `crates/harnesses/tests/candidate_zcode_boundary.rs` only if an official,
  redirectable deterministic validation boundary is found.
- `crates/test-support/src/harness_profile.rs` only after exact version bytes and
  isolated layout are proven.

**Story**: `epic-expanded-harness-support-candidate-admission-zcode-boundary`

Validation must retain the documented global `~/.zcode/skills` contract while
identifying the exact current-project skill destination, exact user/workspace
MCP files, direct-edit support, symlink sibling behavior, enablement/precedence,
and effective reload observation. Import UI destinations are leads, not path
authority.

**Acceptance criteria**:

- [ ] Exact detection/version identity, project skill path, both MCP files, and
      supported observation/reload mechanism are source-bound and reproduced in
      isolated roots.
- [ ] Symlink and copy modes are distinguished; admission uses the mode that
      preserves complete siblings and ownership-safe update/removal through the
      shared project-skill contract.
- [ ] Direct edits are proven supported and effective rather than merely visible
      in an import/settings UI.
- [ ] The story concludes `admitted`, `observe_only`, or `blocked` without
      inventing a `.zcode` filename.

---

### Unit 4: Cursor boundary validation

**Files**:

- Story evidence in
  `.work/active/stories/epic-expanded-harness-support-candidate-admission-cursor-boundary.md`.
- `crates/harnesses/tests/candidate_cursor_boundary.rs` only when the exact CLI
  version and isolated root overrides are established.
- `crates/test-support/src/harness_profile.rs` only after exact version bytes and
  layout are proven.

**Story**: `epic-expanded-harness-support-candidate-admission-cursor-boundary`

Validation preserves the already-attested MCP files and `cursor-agent mcp`
observation while closing the global/project Agent Skills paths, whole-directory
behavior, precedence/reload, and exact version profile. It must prove the editor
and CLI consume the same promised skill surface or describe any divergence
explicitly.

**Acceptance criteria**:

- [ ] `~/.cursor/mcp.json` and `<project>/.cursor/mcp.json` merge, precedence,
      reload/list/tools, update/removal, and unknown-server preservation pass in
      isolated roots.
- [ ] Exact documented skill roots load `SKILL.md`, nested references, scripts,
      executable intent, and updated siblings at both scopes.
- [ ] CLI/editor divergence, OAuth state, extension registration state, and
      caches remain explicit and outside managed ownership.
- [ ] The story concludes `admitted`, `observe_only`, or `blocked`; known MCP
      paths alone cannot grant admission while skill paths remain unresolved.

---

### Unit 5: Zoo Code disposition and optional adapter admission

**Files on `admitted`**:

- `crates/harnesses/src/adapters/zoo.rs` and `zoo_managed.rs` (new).
- Adapter module exports and one canonical registry entry.
- Target-owned MCP codec/effective decoder tests.

**Files on `observe_only`**:

- `crates/harnesses/src/adapters/zoo.rs` with detection, read-only observation,
  and a verified-observe-only profile only.
- Adapter module exports and one canonical registry entry.

**Files on `blocked`**: no production adapter or registry edit; only this story's
recorded disposition and absence assertions in the final acceptance checkpoint.

**Story**: `epic-expanded-harness-support-candidate-admission-zoo-admission`

```rust
pub struct ZooAdapter;
pub struct ZooSkillProjection;       // admitted only
pub struct ZooManagedProjection;     // admitted only
pub struct ZooEffectiveStateProbe;   // admitted or read-only when validated

impl HarnessAdapter for ZooAdapter {
    fn identity(&self) -> TargetIdentity;
    fn version_arguments(&self) -> Vec<OsString>;
    fn decode_version(&self, stdout: &[u8]) -> Result<NativeVersion, DetectionError>;
    fn select_profile(&self, version: &NativeVersion) -> CapabilityProfileSelection;
    fn observe(&self, paths: &PlatformPaths, scope: &Scope, limits: ExternalTreeLimits)
        -> Result<AdapterObservationPaths, ObservationPathError>;
    // Mutating ports exist only for an admitted disposition.
}
```

An admitted implementation uses the boundary story's exact paths, the shared
scope-aware managed projection helper, the existing project-skill link planner,
and a Zoo-owned `mcpServers` codec. An observe-only implementation returns no
mutating ports and no native lifecycle. A blocked result adds no adapter.

**Acceptance criteria**:

- [ ] The implementation exactly matches the boundary disposition; no weaker
      evidence is promoted during coding.
- [ ] Admitted profiles pass both shared matrices and cache-path non-mutation;
      unknown versions remain observe-only.
- [ ] Observe-only profiles expose no mutation capabilities or mutating ports.
- [ ] Blocked disposition leaves `TargetRegistry::canonical()` and CLI target
      help unchanged for `zoo`.

---

### Unit 6: ZCode disposition and optional adapter admission

**Files on `admitted`**:

- `crates/harnesses/src/adapters/zcode.rs` and `zcode_managed.rs` (new).
- Adapter module exports and one canonical registry entry.
- Target-owned skill/MCP codec/effective decoder tests.

**Files on `observe_only`**: one read-only `zcode.rs` adapter and canonical
registry entry. **Files on `blocked`**: no production adapter or registry edit.

**Story**: `epic-expanded-harness-support-candidate-admission-zcode-admission`

```rust
pub struct ZCodeAdapter;
pub struct ZCodeSkillProjection;       // admitted only
pub struct ZCodeManagedProjection;     // admitted only
pub struct ZCodeEffectiveStateProbe;   // admitted or read-only when validated
```

The adapter uses only the exact boundary-story files. Project standalone skills
flow through `project_skill_projection`; the adapter does not invoke ZCode's
import UI or create a second copy/symlink lifecycle. The MCP codec edits only the
verified native server container and preserves unrelated/unknown fields.

**Acceptance criteria**:

- [ ] Admitted implementation preserves source/copy/symlink identity,
      enablement, scope precedence, and owned removal without touching import
      databases or caches.
- [ ] The exact mutable profile passes both scopes, effective reload, drift,
      recovery, removal, and immediate-repeat acceptance.
- [ ] Observe-only and blocked outcomes have the same authority/registry safety
      guarantees as Zoo.

---

### Unit 7: Cursor disposition and optional adapter admission

**Files on `admitted`**:

- `crates/harnesses/src/adapters/cursor.rs` and `cursor_managed.rs` (new).
- Adapter module exports and one canonical registry entry.
- Cursor MCP codec/effective decoder tests.

**Files on `observe_only`**: one read-only `cursor.rs` adapter and canonical
registry entry. **Files on `blocked`**: no production adapter or registry edit.

**Story**: `epic-expanded-harness-support-candidate-admission-cursor-admission`

```rust
pub struct CursorAdapter;
pub struct CursorSkillProjection;       // admitted only
pub struct CursorManagedProjection;     // admitted only
pub struct CursorEffectiveStateProbe;   // wraps validated cursor-agent MCP observation
```

An admitted Cursor adapter edits only `mcpServers` in the two attested
`mcp.json` files and projects complete skills into the newly validated roots.
`cursor-agent mcp list`/tool inspection is executed through the bounded process
port with explicit scope cwd and exact-version decoding. OAuth and extension
registration APIs remain native/user-owned.

**Acceptance criteria**:

- [ ] Exact profile and both scope paths come from the boundary story rather
      than inferred Cursor conventions.
- [ ] `mcp.json` unknown fields/unmanaged servers survive, same-name unowned
      servers conflict, and CLI effective state matches declared state after
      reload.
- [ ] Whole skill trees and project links/canonical no-ops pass the shared
      acceptance contract.
- [ ] Observe-only and blocked outcomes expose no latent mutation route.

---

### Unit 8: Aggregate candidate isolation and acceptance

**Files**:

- `crates/test-support/src/candidate_admission.rs` — concrete report/profile
  constructors for whichever candidate dispositions were established.
- `crates/harnesses/tests/detection.rs` and target contract tests — exact profile
  authority and unknown-version behavior for registered candidates.
- `crates/cli/tests/compiled_binary.rs` — registry/help/list/status/plan/sync and
  target-isolation behavior.
- `crates/cli/src/application/tests.rs` — managed acceptance runner entries for
  admitted candidates only.

**Story**: `epic-expanded-harness-support-candidate-admission-acceptance`

**Implementation notes**:

- Registry order adds only registered candidates and remains the single source
  for help, config membership, enablement, `--target all`, status labels, and
  dispatch. First-party bootstrap remains Codex/Claude only.
- Every candidate receives one final disposition assertion:
  - admitted: registered exact mutable profile, both shared matrices pass;
  - observe-only: registered verified-observe-only profile, status works, every
    mutation command returns attention before writes;
  - blocked: absent from registry/help/config mutation and no candidate path
    constant or adapter module exists.
- Run mixed-target tests proving an admitted candidate mutation updates only its
  target-local state and cannot grant a sibling candidate authority.
- Use isolated HOME/XDG/editor/project roots and fake executables/extension hosts
  only. Real native boundary validation belongs in Units 2–4 and is not silently
  replaced by fake evidence.

**Acceptance criteria**:

- [ ] Each candidate has exactly one grounded disposition and the production
      registry/profile/port shape matches it.
- [ ] Every admitted candidate passes detection, both scopes, complete skills,
      MCP schema/precedence/secrets, effective reload, ownership/drift,
      update/removal, pending recovery, partial/required compatibility, and
      immediate-repeat acceptance.
- [ ] Every observe-only candidate is inspectable but cannot plan or apply a
      mutation for any scope or resource kind.
- [ ] Every blocked candidate remains unregistered and produces no guessed
      filesystem surface.
- [ ] Selecting one candidate preserves every sibling target's desired/state
      binding and cannot widen its capability profile.
- [ ] Native editor caches, extension storage, credentials, and operator roots
      remain byte-for-byte untouched.
- [ ] Workspace tests, all-feature Clippy with warnings denied, formatting, and
      `git diff --check` pass before feature review.

## Implementation Order

1. `epic-expanded-harness-support-candidate-admission-gate` — Unit 1,
   `depends_on: []`.
2. Boundary tracks are dependency-parallel after the gate:
   - `...-zoo-boundary` — Unit 2, `depends_on: [...-gate]`.
   - `...-zcode-boundary` — Unit 3, `depends_on: [...-gate]`.
   - `...-cursor-boundary` — Unit 4, `depends_on: [...-gate]`.
   One cohesive owner executes Zoo first (highest uncertainty), then ZCode, then
   Cursor, while the graph preserves their independence.
3. Target dispositions consume their own boundary result plus the shared
   scope-aware file-managed contract:
   - `...-zoo-admission` — Unit 5, `depends_on: [...-zoo-boundary,
     epic-expanded-harness-support-file-managed-contracts]`.
   - `...-zcode-admission` — Unit 6, `depends_on: [...-zcode-boundary,
     epic-expanded-harness-support-file-managed-contracts]`.
   - `...-cursor-admission` — Unit 7, `depends_on: [...-cursor-boundary,
     epic-expanded-harness-support-file-managed-contracts]`.
4. `epic-expanded-harness-support-candidate-admission-acceptance` — Unit 8,
   `depends_on` all three admission stories.

Cycle checks with `.work/bin/work-view --blocking <story-id>` returned no
existing edges for all eight proposed story ids. The external
`epic-expanded-harness-support-file-managed-contracts` story is upstream-only;
it does not depend on this feature or its children.

## Simplification

- Add one accurate verified-observe-only profile variant instead of encoding a
  known candidate as `UnknownVersion` or a mutation-authorized empty profile.
- Reuse the registry, scoped managed projection, effective probe, project-skill
  projection, target-local state, and shared matrices. Do not add candidate
  dispatch, editor-family adapters, or per-target executors.
- Keep validation evidence in the relevant story body and executable contract
  tests. Do not create parallel research, design, or progress documents.
- Do not add empty candidate modules, guessed path constants, dormant codecs, or
  fake fixture profiles for blocked candidates.
- If a candidate is admitted, reuse the file-managed source-side helper and
  implement only target-owned paths/schema/reload logic.
- If a candidate is observe-only, omit every mutation port rather than adding
  runtime `if candidate` checks.
- Replace no existing tests solely to increase candidate counts. Extend the
  shared matrices and retain Codex/Claude/direct-adapter regressions.

No separate cleanup/refactor story is warranted; every elimination is part of
the gate or target admission checkpoint that verifies it.

## Testing

- **Profile authority tests:** verified-observe-only serialization, profile id,
  observation capability, runtime narrowing, and guaranteed absence of mutation
  capabilities. Protects the core admission invariant.
- **Native boundary tests:** explicit opt-in tests against isolated real
  candidates, only when every root and process is redirectable. Protects exact
  path/version/reload claims; an unavailable safe harness yields a blocked
  disposition, not a skipped green test.
- **Codec tests for admitted candidates:** preserve unknown document fields and
  unowned sibling servers; reject malformed/duplicate containers, same-name
  conflicts, unsupported transports/auth, and literal secret acquisition.
  Protects ownership-safe MCP edits.
- **Project-skill tests:** exact native root becomes canonical no-op or the
  existing relative per-skill link; complete siblings and executable intent
  remain reachable; update/removal use confined link identity. Protects the
  review-ready project link contract.
- **Managed acceptance:** both scopes, one checkout, complete tree, manifest and
  aggregate fingerprints, omission acknowledgment, required blocking,
  ownership/drift, target-local state, pending recovery, fresh effective load,
  removal, and immediate repeat. Protects adapter integration.
- **Disposition tests:** admitted, observe-only, and blocked registry/help/plan
  behavior. Protects honest product claims and target isolation.
- **Test economy:** do not test trivial adapter getters or every UI label. Pin
  stable ids, exact version bytes, paths, file containers, finding codes,
  capability authority, effective state, ownership behavior, and user-visible
  outcomes.

## Risks

- **Zoo may have no supported cache-independent observation boundary.** Its
  attested global MCP surface is UI-opened and the product is extension-backed.
  If official platform paths or deterministic reload/effective observation are
  absent, Zoo closes `blocked` or `observe_only`; the design does not substitute
  VS Code storage mutation or screen scraping.
- **ZCode may expose import behavior without direct-write authority.** A visible
  `.zcode` family or symlink created by the UI is not proof of a stable native
  file contract. Missing exact filenames or direct reload keeps it unadmitted.
- **Cursor editor and CLI may not share skill precedence/reload.** MCP evidence
  is stronger than skill-path evidence. Divergence must be modeled explicitly;
  if both promised surfaces cannot be reconciled faithfully, Cursor remains
  observe-only rather than advertising partial mutable support.
- **Real native validation may not be safely isolatable.** Some editor hosts may
  ignore HOME/XDG overrides or retain machine credentials. Any probe that can
  touch operator state is prohibited and results in a blocked disposition.
- **Official contracts may move between design and implementation.** Boundary
  stories re-check current primary sources and pin exact versions before adding
  constants. Unknown/new versions remain observe-only.
- **The scope-aware managed/effective contract is in flight.** Admission stories
  explicitly depend on `epic-expanded-harness-support-file-managed-contracts`.
  If its realized API differs from its design, candidate stories adapt to the
  reviewed implementation rather than creating a competing contract.
- **A verified-observe-only variant broadens a serialized boundary.** It must be
  handled by every exhaustive profile renderer and round-trip test. Because it
  is ephemeral capability evidence rather than persisted desired/apply state,
  no inventory/state schema bump is expected; implementation must verify that
  assumption before landing.
- **Conditional story outcomes complicate acceptance.** The aggregate checkpoint
  requires a single explicit disposition per target and tests the corresponding
  production shape, preventing a blocked candidate from being mistaken for
  omitted work.
