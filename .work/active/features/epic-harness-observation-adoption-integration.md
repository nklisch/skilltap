---
id: epic-harness-observation-adoption-integration
kind: feature
stage: done
tags: [testing]
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-adopt]
release_binding: 3.0.0
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Observation and Adoption End-to-End Contracts

Exercise compiled CLI flows against scripted Codex/Claude executables and real
isolated native trees. Cover absent/unknown/malformed/failing harnesses; bounded
process and JSON failures; global/current/explicit/all scopes; CODEX_HOME;
declared/effective/cache/trust distinctions; whole skills and symlink/race
edges; cross-scope and qualified identity; partial sibling success; conflicts;
safe diagnostics; first-use no-create; native byte/type/link/mtime no-mutation;
locked inventory-only adoption; and immediate repeated observation/adoption
with no changes. Run Linux and native macOS contracts.

## Architectural choice

Use a layered contract suite rather than one giant scenario: deterministic
fixture builders in `skilltap-test-support`, core seam tests for conflict and
lock behavior, and compiled-binary tests for real scope resolution, output,
and filesystem non-mutation. This keeps failures localized and lets the same
fixtures run on Linux and macOS while preserving the native process and
filesystem boundaries. A single end-to-end test would be shorter, but would
hide which contract failed and make platform-specific diagnosis difficult.

## Design decisions

- **Cross-platform execution**: use the existing isolated-machine and scripted
  native-process fixtures; assert portable contracts on every host and gate
  platform-specific path assertions behind the existing runtime platform
  helpers.
- **Native mutation boundary**: snapshot native trees, owned config/state, and
  symlink metadata before adoption; after every successful and repeated run,
  assert only `inventory.toml` changed.
- **Partial observations**: preserve healthy siblings and return attention;
  never publish a candidate whose selected evidence was stale or unavailable.
- **Coverage ownership**: reuse the completed runtime, detection, adapter,
  status, and adoption tests; this feature adds only missing seams and matrix
  coverage rather than duplicating unit contracts.

## Implementation Units

### Unit 1: Deterministic integration fixtures

**File**: `crates/test-support/src/integration.rs` (new, exported from
`crates/test-support/src/lib.rs`)

```rust
pub struct HarnessFixture {
    pub machine: IsolatedMachine,
    pub codex: Option<FakeNativeProcess>,
    pub claude: Option<FakeNativeProcess>,
}

pub struct NativeTreeSnapshot { /* bounded bytes, types, links, mtimes */ }

pub fn snapshot_native_roots(machine: &IsolatedMachine) -> NativeTreeSnapshot;
pub fn write_enabled_config(machine: &IsolatedMachine, codex: &Path, claude: &Path);
```

**Acceptance criteria**:

- Fixtures create complete skill directories containing top-level `SKILL.md`,
  plugin/config/cache/trust examples, symlinks, and a malformed sibling.
- Snapshots never follow links and compare bytes, entry kinds, link targets,
  and mtimes deterministically.

### Unit 2: Core adoption seam matrix

**Files**: `crates/core/src/adoption.rs` tests and
`crates/core/tests/storage_integration.rs`

```rust
fn assert_conflict_isolated_from_unrelated_candidate();
fn assert_declared_only_and_unresolved_are_attention_findings();
fn assert_lock_reload_and_stale_fingerprint_publish_zero_writes();
```

**Acceptance criteria**:

- Equivalent Codex/Claude candidates coalesce only on exact key and semantics;
  different semantics remain conflicts and unrelated additions survive.
- Declared-only, unresolved, shared-scope, partial-sibling, and unknown
  observations are explicit attention results, never invented desired state.
- Lock contention, manual inventory edits, changed native identity, and changed
  fingerprint all fail before replacement; repeat application is a no-op.

### Unit 3: Compiled CLI contract matrix

**File**: `crates/cli/tests/compiled_binary.rs`

```rust
#[test] fn adopt_scope_and_target_matrix_is_exact();
#[test] fn adopt_conflict_and_partial_sibling_output_is_stable();
#[test] fn adopt_changes_inventory_only_and_repeats_without_rewrite();
```

**Acceptance criteria**:

- Global, current project, explicit project, all recorded scopes, omitted
  target, and `--from` target selection match the documented command contract.
- Plain and JSON output share typed decisions, safe diagnostics, attention exit
  classes, and no generic `--yes` bypass.
- Native trees, config, state, symlink types, bytes, and mtimes remain unchanged;
  only inventory is created/replaced for a new non-conflicting candidate.

### Unit 4: Platform contract guardrails

**Files**: `crates/core/tests/runtime_integration.rs`,
`crates/cli/tests/compiled_binary.rs`

```rust
#[test] fn codex_home_and_claude_home_never_cross_observation_roots();
#[test] fn bounded_native_failures_remain_secret_safe_and_non_hanging();
```

**Acceptance criteria**:

- Linux and macOS path contracts use the platform resolver rather than host
  assumptions.
- Missing, unknown, malformed, non-zero, hanging, and flood fixtures terminate
  within configured bounds and do not leak raw output or secrets.

## Implementation Order

1. `epic-harness-observation-adoption-integration-fixtures` — shared fixture
   and snapshot helpers.
2. `epic-harness-observation-adoption-integration-core` — pure and storage seam
   matrix; depends on fixtures.
3. `epic-harness-observation-adoption-integration-cli` — compiled command and
   no-mutation matrix; depends on core.
4. `epic-harness-observation-adoption-integration-platform` — portable runtime
   guardrails; depends on CLI.

## Testing

Run each story's focused tests first, then `cargo test --workspace --all-targets
--offline` and `cargo clippy --workspace --all-targets --offline -- -D
warnings`. Any platform-only assertion must be skipped or adapted through the
existing `SupportedPlatform` abstraction, never by assuming Linux paths.

## Risks

- Native Claude/Codex manifests are not yet parsed into per-resource lineage;
  integration tests must assert bounded surface behavior and record deeper
  parsing as follow-up rather than manufacturing identities.
- Lock contention and filesystem race tests can be timing-sensitive; use the
  existing barriers and deterministic lock helpers instead of sleeps.
- Native macOS CI may be unavailable locally; keep the portable contract suite
  runnable here and document any platform lane not exercised in this run.

## Implementation notes

- Added reusable no-follow native-tree snapshots to test support.
- Expanded core adoption seams for declared-only candidates and lock
  contention, compiled CLI coverage for partial sibling, project/all-scope, and
  repeat adoption, and bounded flood-output diagnostics.
- Existing runtime and native-process fixtures continue to cover CODEX_HOME,
  platform path resolution, hangs, termination, and repeatable observations.

## Review

### Verdict

Approve with comments.

### Findings

- Important: native macOS execution was not available in this Linux run; the
  portable platform contracts are covered and the native lane remains a CI
  responsibility.
- Important: deeper per-plugin/per-skill manifest lineage and shared-Claude
  declaration semantics remain follow-up work; current tests correctly assert
  bounded surface observation and explicit attention rather than inventing
  equivalence.

### Notes

All four integration stories passed their focused verification. Full workspace
verification is required before the parent epic advances.
