---
id: epic-real-harness-recovery-and-adapter-expansion
kind: epic
stage: implementing
tags: [correctness, testing, architecture]
parent: null
depends_on: []
release_binding: null
research_refs:
  - .research/analysis/briefs/harness-adapter-candidates-2026-07-12.md
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Restore real harness operation and expand adapter eligibility

## Brief

Repair every defect found by the isolated real-Codex, real-Claude, and blind
clean-room validation pass, then re-evaluate target harnesses under the revised
minimum contract. A target harness no longer needs a native marketplace or
plugin lifecycle: skilltap may own source acquisition, installation, update,
and removal through documented load paths. Target eligibility requires faithful
whole-directory skill loading and MCP support. Hooks and all other components
are optional capabilities whose absence is reported as a compatibility
consequence.

The outcome is not complete until current real Codex and Claude binaries work
inside disposable homes, the complete fixture suite still passes, every
mutating workflow repeats as a no-op, and the refreshed target-harness research
identifies candidates and exclusions under the new bar.

## Strategic decisions

- **Minimum target contract:** faithful whole-directory skills plus MCP loading
  and observation. Native marketplace and plugin lifecycle are optional.
- **Ownership when native lifecycle is absent:** skilltap owns acquisition,
  managed projection, update, drift detection, and removal without writing
  undocumented caches.
- **Native lifecycle when available:** prefer and track the native installation
  independently for every target; never replace a dual-native plugin with a
  skilltap-managed copy.
- **Optional components:** detect hooks, instructions, agents, commands, and
  other component capabilities. Missing optional behavior is partial and
  requires the normal disclosed acknowledgment; missing required behavior
  remains blocked.
- **Evidence bar:** real installed CLIs in isolated environments are required
  alongside fake fixtures. Synthetic version `3.0.0` fixtures alone are not
  release evidence.

## Verified blocker inventory

### Detection, capability, and process isolation

1. Codex 0.144.1 and Claude 2.1.201 emit plain text for `--version --json`, but
   detection requires `{ "version": ... }`; both are reported unreachable.
2. Mutation authority recognizes only fictitious exact native version `3.0.0`.
3. Native subprocesses clear `HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`,
   `CODEX_HOME`, `CLAUDE_CONFIG_DIR`, and `PATH`; this breaks configured roots
   and can escape an isolation boundary.
4. Detection errors collapse successful-but-unparseable version output into a
   generic unreachable result without an actionable boundary reason.

### Native lifecycle contracts

5. Claude `plugin list --json` and `plugin marketplace list --json` reject the
   generated `--scope` flag.
6. Claude `plugin marketplace update` rejects the generated `--scope` flag.
7. Codex 0.144.1 has no `plugin update` command; update must use an attested
   replacement lifecycle or report the capability unavailable.
8. `CLAUDE_CONFIG_DIR` is honored by Claude but ignored by skilltap observation.
9. Codex project mutations stop at an unverified native capability instead of
   selecting the documented managed load-path fallback.
10. Successful native mutations can end as generic observation failures with
    no useful adapter-level cause.

### Bootstrap and updates

11. The release fetcher writes `http_code` followed by an empty redirect URL;
    line reversal treats `200` as the redirect and reports
    `release_manifest_failed` on a successful response.
12. `available_updates` counts unresolved and blocked candidates as available
    updates, including local instructions and local-path skills.

### Filesystem and instructions

13. Custom `CODEX_HOME` receives a fixed `../AGENTS.md` link that can resolve
    outside `$HOME`; status and repair still classify it as managed.
14. Whole-skill publication preserves files but strips executable bits from
    scripts.
15. An acknowledged divergent instruction repair applies and creates a backup
    but still exits as attention-required despite no remaining blocker.

### CLI and state semantics

16. `plugin remove` help describes a plugin name while parsing requires the
    exact `plugin@marketplace` selector.
17. Repeated identical next actions are emitted once per operation rather than
    deduplicated.
18. Installing a plugin for one target and later adding the sibling target can
    reuse the old desired resource without widening its target set.
19. Dual-native state has per-target native IDs but resource-wide source,
    revision, provenance, and ownership; differing native resolutions cannot be
    explained per target.
20. Exact dual-native coverage must assert target-all install/update/removal,
    narrowed sibling preservation, no managed plugin artifact, repeat no-ops,
    and target-specific provenance evidence.

## Acceptance

- Every blocker above has a regression test tied to its public behavior.
- Current installed Codex and Claude versions are detected and capability-
  probed without invented version aliases or permissive unknown-version
  mutation.
- Native child processes receive an explicit minimal safe environment that
  preserves configured roots and does not inherit unrelated secrets.
- Real native command vectors match current CLI help and isolated execution.
- Bootstrap succeeds against a real successful release response and preserves
  redirect validation.
- Whole skill directories preserve required executable semantics safely.
- Instruction bridges resolve to the canonical file for arbitrary supported
  `HOME` and `CODEX_HOME` relationships.
- JSON/result/next-action semantics distinguish completed acknowledged work,
  blocked updates, and precise native contract failures.
- Dual-native plugins remain native in both harnesses and carry per-target
  lifecycle evidence without managed copies.
- The refreshed research report applies the skills-plus-MCP eligibility bar and
  supplies attested candidates, capability gaps, and implementation order.
- Full workspace, website, install-surface, release-contract, isolated native,
  idempotence, formatting, and strict clippy checks pass.

## Design decisions

- Keep native-command authority and managed-publication authority separate:
  version/profile evidence gates the former; attested filesystem load paths,
  ownership, and drift checks gate the latter.
- Treat the explicit child-process environment as part of the adapter contract,
  not an incidental inheritance setting. This preserves isolation without
  forwarding unrelated secrets.
- Split repairs by capability boundary rather than Rust crate so each feature
  owns an observable outcome and can be verified end to end.
- Put dual-native provenance and output aggregation after native lifecycle and
  filesystem result semantics so it cannot normalize away unresolved adapter
  failures.

## Decomposition

The epic is split into three independent foundation repairs, one native adapter
consumer, one final state/diagnostics integrator, and one independent research
input. Runtime detection and process isolation unlock real native testing;
bootstrap transport and filesystem/instruction correctness can proceed in
parallel. Native lifecycle follows runtime, while target-exact state and
diagnostics follow both native lifecycle and filesystem result semantics.

### Child features

- `epic-real-harness-recovery-runtime-boundary` — detect and safely execute
  current real harnesses and resolve configured roots — depends on: `[]`
- `epic-real-harness-recovery-bootstrap-transport` — repair verified release
  response handling — depends on: `[]`
- `epic-real-harness-recovery-filesystem-instructions` — preserve executable
  skill files and correct arbitrary-root instruction bridges — depends on:
  `[epic-real-harness-recovery-runtime-boundary]`
- `feature-relaxed-target-harness-research` — reassess adapters under the
  skills-plus-MCP admission bar — depends on: `[]`
- `epic-real-harness-recovery-native-lifecycle` — align native command vectors,
  roots, scope, and fallbacks — depends on:
  `[epic-real-harness-recovery-runtime-boundary]`
- `epic-real-harness-recovery-state-diagnostics` — make updates, outputs, and
  dual-native state target-exact — depends on:
  `[epic-real-harness-recovery-native-lifecycle,
  epic-real-harness-recovery-filesystem-instructions]`

### Decomposition risks

- Current fixtures encode synthetic version and command contracts; feature
  verification must use real CLI help and isolated real execution without
  broadening unknown-version mutation.
- Environment repair can accidentally inherit secrets or escape isolation if
  it forwards the parent wholesale; the contract must use an explicit minimal
  allowlist.
- Preserving executable bits can conflict with private managed-file defaults;
  only source-executable regular files should retain execution, with write and
  special-mode bits normalized safely.
- Per-target provenance changes the persisted wire contract and must preserve
  validation, existing state semantics, and independent schema discipline.
