---
id: epic-harness-observation-adoption
kind: epic
stage: implementing
tags: []
parent: null
depends_on: [epic-rust-control-plane]
release_binding: null
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
  - .research/analysis/campaigns/marketplace-standards/specialists/codex.md
  - .research/analysis/campaigns/marketplace-standards/specialists/claude.md
  - .research/analysis/campaigns/marketplace-standards/specialists/agent-skills.md
research_origin: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Harness Observation and Adoption

## Brief

Make Codex and Claude Code observable without changing their environments.
This epic delivers harness detection, runtime-versioned capability profiles,
native configuration and resource observation, normalized identities, health
findings, first-use status, and explicit adoption into desired inventory.

Observation remains read-only and reports malformed, unmanaged, conflicting,
or unknown-version state instead of hiding it. Adoption records non-conflicting
native resources and provenance but does not transfer or mutate them.

## Foundation references

- `docs/SPEC.md` — Harness Commands, Adoption, Status
- `docs/ARCH.md` — Harness Adapter Contract, Capability Detection, Observation, Adoption Flow
- `docs/HARNESS-CONTRACTS.md` — Common Capability Model, Codex Contract, Claude Code Contract
- `docs/UX.md` — First Use, Enabling Harnesses, Adoption, Status

## Design decisions

- **How are harness capability profiles updated?** Compile verified profiles
  into skilltap and update them through ordinary skilltap releases. Runtime
  probes may confirm or narrow a compiled profile, but never grant undocumented
  mutation authority. Unknown harness versions remain observable when their
  state is parseable and stay mutation-blocked until a verified profile ships.
- **Does this epic require UI mockups?** No. Harness status and adoption are
  non-interactive CLI surfaces represented through plain and JSON output.

- **What grants mutation authority?** Compiled, versioned profiles are the
  allowlist. Runtime probes confirm or narrow a profile; they never grant an
  undocumented mutation capability. Unknown versions may be observed only
  through attested tolerant parsers and documented files, with every mutation
  capability unverified. This resolves the looser wording currently present in
  `HARNESS-CONTRACTS.md`, which the contracts feature must align.
- **How does first use treat missing config?** Missing configuration is an
  explicit state, not `ConfigDocument::defaults()`. Status detects both known
  harnesses but reports neither as user-enabled. `harness enable <id>` creates
  config with only that harness enabled; a present config remains authoritative.
- **How are resource identity and revision separated?** A stable logical
  `ResourceId` plus concrete scope key identifies a resource instance. Mutable
  native version, resolved revision, fingerprint, and declared/effective layer
  are observations, never identity. User selectors may use the documented
  qualified `plugin:name@marketplace` spelling.
- **Are fresh observations persisted?** No. Status and the shared observation
  service are read-only. This epic keeps declared/effective observations and
  findings ephemeral; adoption persists desired inventory only, with
  `DesiredOrigin::Adopted(source_harness)` as provenance. Later successful
  mutation workflows own state snapshots and apply history.
- **What happens to shared Claude project declarations?** They are observed as
  declared state and health evidence but are not adoptable into personal local
  scope because the current CLI has no explicit shared-scope adoption selector.

## Decomposition

The epic starts by correcting normalized observation identity and safety
contracts. Native process and filesystem reads then provide a bounded substrate
for detection and profile selection. Codex and Claude adapters proceed in
parallel behind the same port. A normalization coordinator combines successful
siblings without hiding per-harness failures. User-facing harness/status flows
follow, then locked conflict-aware adoption and a final end-to-end contract.

### Child features

1. `epic-harness-observation-adoption-contracts` — scope-bearing resource keys,
   safe observation/install/profile/finding contracts, adapter/coordinator
   ports, ephemeral snapshot semantics, and foundation alignment — depends on
   `[]`.
2. `epic-harness-observation-adoption-runtime` — bounded native execution,
   executable identity, strict structured-output boundaries, `CODEX_HOME`, and
   bounded no-follow external-state traversal — depends on
   `[epic-harness-observation-adoption-contracts]`.
3. `epic-harness-observation-adoption-detection` — registry, installation
   detection, compiled scoped profiles, read-only narrowing probes, and common
   native contract fixtures — depends on
   `[epic-harness-observation-adoption-contracts,
   epic-harness-observation-adoption-runtime]`.
4. `epic-harness-observation-adoption-codex` — Codex declared/effective
   marketplace, plugin, skill, instruction, config/trust, and cache observation
   — depends on `[epic-harness-observation-adoption-detection]`.
5. `epic-harness-observation-adoption-claude` — Claude user/local/project/
   managed marketplace, plugin, skill, instruction, consent, settings, and
   cache observation — depends on
   `[epic-harness-observation-adoption-detection]`.
6. `epic-harness-observation-adoption-normalization` — stable native lineage,
   declared/effective correlation, conservative cross-harness association,
   unresolved dependency and health findings, and partial-success aggregation
   — depends on `[epic-harness-observation-adoption-codex,
   epic-harness-observation-adoption-claude]`.
7. `epic-harness-observation-adoption-status` — harness list/enable/disable,
   explicit missing-config policy, shared observation-backed first-use status,
   scope/target expansion, and CLI rendering — depends on
   `[epic-harness-observation-adoption-normalization]`.
8. `epic-harness-observation-adoption-adopt` — pure candidate/coalescing/
   conflict logic and locked incremental inventory persistence with source
   provenance and idempotent CLI behavior — depends on
   `[epic-harness-observation-adoption-status]`.
9. `epic-harness-observation-adoption-integration` — fake-native and real
   filesystem end-to-end no-mutation, first-use, partial-failure, conflict, and
   repeat-adoption contracts — depends on
   `[epic-harness-observation-adoption-adopt]`.

## Pre-mortem

- **A probe hangs or floods status.** Every native invocation has a deadline,
  bounded stdout/stderr, null stdin, kill-and-reap behavior, direct arguments,
  and sanitized failures before adapters exist.
- **A replaced PATH executable inherits trusted evidence.** Resolve one
  canonical executable identity for a snapshot and bind version/profile/probe
  evidence to it; later mutation must revalidate it.
- **Native cache walking escapes or blocks.** Use a read-only bounded walker
  with directory identities, no-follow entry inspection, entry/depth/byte
  limits, and typed symlink/non-regular findings; never reuse cache writes.
- **Declared state is mistaken for effective install.** Preserve both layers
  and correlate only with explicit native lineage; settings, catalogs, and
  caches are independent evidence.
- **Resource instances collide across projects.** Use scope-bearing keys in
  graphs, inventory, and state while keeping logical selectors separate.
- **Native data leaks secrets.** Findings use authored codes/summaries and
  typed sanitized fields only; raw argv, output, settings, unknown JSON, and
  symlink targets never enter display, JSON, or state.
- **One malformed resource hides healthy siblings.** Adapter parsers return
  per-entry findings and partial snapshots; the coordinator preserves every
  successful harness/scope.
- **Adoption races manual inventory edits.** Acquire the configuration lock,
  reload inventory, revalidate selected observations, merge non-conflicting
  candidates incrementally, publish one atomic inventory replacement, and
  make immediate repetition a no-op.

Each child feature's design pass owns exact schemas, fixture inventories, and
story decomposition within these boundaries.
