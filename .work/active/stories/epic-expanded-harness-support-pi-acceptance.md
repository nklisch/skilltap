---
id: epic-expanded-harness-support-pi-acceptance
kind: story
stage: done
tags: []
parent: epic-expanded-harness-support-pi
depends_on: [epic-expanded-harness-support-pi-integration]
release_binding: null
research_refs:
  - .research/analysis/campaigns/pi-claude-hook-compatibility/parent.md
research_origin: operator-request-2026-07-12
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Verify Pi Conditional-Target Acceptance

## Checkpoint

Prove the registered Pi target provides deterministic, component-separated
observation and safe observe-only behavior at both scopes without touching the
operator's real Pi installation or claiming mutable-adapter acceptance.

## Fixture contract

- Extend `FakeHarnessProfile` through profile data with exact
  `pi --version -> 0.80.6`, no lifecycle dialect, and isolated Pi/package/
  skill/MCP/settings roots. Do not add target-id layout branching.
- Add settings and manifest fixtures for `pi-mcp-adapter@2.11.0` and
  `@hsingjui/pi-hooks@0.0.2`, project/global precedence, hook configuration,
  trust uncertainty, malformed boundaries, and unknown versions.
- Use only `IsolatedMachine` HOME/XDG/package/project/fake binary roots. Never
  inspect or mutate host `~/.pi`, repository `.pi/`, package checkouts, or a
  real `pi` binary.
- Snapshot every native surface and target-state document around blocked
  commands; verify byte identity and absence of Pi ownership/journal/pending
  evidence.

## Acceptance matrix

The mutable managed-adapter matrix is intentionally inapplicable. The
conditional-target matrix proves:

- exact core/component identity and version observation;
- independent presence, activation, compatibility, trust, and ownership;
- compiled tuple plus narrowing behavior;
- non-adoption of companions;
- mutation denial before execution;
- safe progress for unrelated targets;
- deterministic read-only repeats.

## Files

- `crates/test-support/src/harness_profile.rs`
- `crates/test-support/src/conditional_profile.rs` (new)
- `crates/harnesses/tests/detection.rs`
- Pi adapter unit/contract tests
- `crates/cli/src/application/tests.rs`
- `crates/cli/tests/compiled_binary.rs`

## Required scenarios

- Exact tuple with absent hooks and no MCP config: core reachable, MCP
  unverified, hook inert/partial, aggregate mutation unavailable.
- Valid hooks config: activation changes to configured-unverified but semantic
  partial and mutation result do not change.
- Missing MCP, missing hook, mismatched package identity, unknown package/core
  version, malformed settings/manifest, and project trust uncertainty each
  produce distinct stable findings without hiding sibling facts.
- Project package precedence and hook concatenation are verified separately.
- Canonical project `.agents/skills` requires no link; `.pi/skills` siblings are
  observed and preserved.
- `adopt --from pi` excludes companions. Skill/plugin/marketplace/plan/sync/
  daemon mutation leaves Pi files and target state unchanged.
- An authorized sibling target applies in the same command while Pi reports
  attention required.
- Plain and JSON output agree on ids, versions, activation, compatibility,
  ownership, profile, warnings, next actions, and result class.
- Immediate status/profile repeats are identical and side-effect-free.

## Implementation evidence

- Added dependency-neutral conditional-target fixtures carrying the exact Pi
  version/layout data and isolated global/project settings, package, skill, and
  sibling roots. The matrix covers exact, configured hooks, missing companions,
  mismatched identity, unknown versions, malformed boundaries, and project trust.
- Added focused adapter, detection, application, and compiled tests for
  component separation, narrowing-only authority, companion non-adoption,
  plain/JSON parity, repeated read-only observation, and byte-preserving
  mutation denial.
- Kept `plan` and empty `sync` as attention/no-op outcomes with zero operations;
  they are read-only or have no selected mutation. Added a reconciliation guard
  for conditional-target instruction sync so a real adopted-resource sync cannot
  write before profile authorization. Daemon bookkeeping is asserted separately
  from Pi native bytes and Pi target state.
- The mutable managed-adapter matrix remains intentionally inapplicable; no
  mutable Pi acceptance claim is made.

## Verification

Focused tests, `cargo test --workspace --all-targets`, strict all-feature
Clippy, `cargo fmt --all -- --check`, and `git diff --check` pass. The child is
done; the parent feature remains untouched for its independent review pass.
