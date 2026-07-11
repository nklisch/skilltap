---
id: epic-safe-update-automation-policy
kind: feature
stage: implementing
tags: []
parent: epic-safe-update-automation
depends_on: [epic-safe-update-automation-resolution, epic-cross-harness-materialization]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Classify Safe Update Policy

Honor pins and update intent, re-evaluate compatibility, and identify plans
that introduce no new user decision.

## Architectural choice

Keep safety classification pure and explicit at the core boundary. The
resolver produces a typed revision candidate; a policy classifier then applies
global `UpdateMode`, per-resource `UpdateIntent`, drift, compatibility deltas,
and acknowledgment requirements. Foreground and daemon callers consume the
same `UpdateDecision`; neither caller infers safety from semver or revision
distance. CLI status renders the decision and reason but does not mutate state.

## Design decisions

- **How does a disabled resource appear?** It is a deterministic `NoUpdate`
  decision with a `disabled_resource` reason, so status can distinguish it from
  an up-to-date resource and automation never attempts resolution application.
- **How do `off` and `check` differ?** `off` suppresses automatic update
  application and resolution checks; `check` permits resolution and reporting
  but classifies an otherwise safe candidate as `NeedsDecision` for mutation.
  `apply-safe` is the only mode that emits `Safe`.
- **What is a new decision?** Any drift, pin, compatibility change, partial or
  acknowledgment requirement, resolver failure, or policy mode that disallows
  automatic application. The classifier preserves the first deterministic
  blocking reason and never downgrades an unsafe candidate because a revision
  looks small.

## Implementation Units

### Unit 1: Typed policy decision (trickiest unit)
**File**: `crates/core/src/updates.rs`
**Story**: `epic-safe-update-automation-policy-contract`

```rust
pub enum UpdateDecisionReason {
    DisabledResource,
    GlobalModeOff,
    CheckOnly,
    PinnedResource,
    Drifted,
    CompatibilityChanged,
    AcknowledgmentRequired,
    ResolutionFailed,
}

pub struct UpdateDecision {
    pub safety: UpdateSafety,
    pub reason: Option<UpdateDecisionReason>,
}

pub fn classify_update_with_mode(
    candidate: &UpdateCandidate,
    mode: UpdateMode,
) -> UpdateDecision;
```

**Implementation Notes**:
- Preserve `classify_update` as the apply-safe compatibility helper while new
  callers use `classify_update_with_mode`.
- Carry `UpdateIntent::Disabled` explicitly rather than treating it as a pin;
  pinned resources remain manually updateable but never auto-safe.
- Resolution failures stay `Blocked` and do not become `NoUpdate`.

**Acceptance Criteria**:
- [ ] Disabled, pinned, drifted, partial, and compatibility-changed candidates
      produce distinct reasons.
- [ ] `off` and `check` never produce an automatically safe decision.
- [ ] A clean tracked candidate in `apply-safe` is the only automatic `Safe`
      result.

### Unit 2: Compatibility decision bridge
**File**: `crates/core/src/updates.rs` and `crates/core/src/reconciliation.rs`
**Story**: `epic-safe-update-automation-policy-compatibility`

```rust
pub struct UpdateChangeSummary {
    pub compatibility_changed: bool,
    pub added_required_components: usize,
    pub partial_components: usize,
    pub requires_acknowledgment: bool,
}

pub fn update_change_summary(
    before: &CompatibilityAnalysis,
    after: &CompatibilityAnalysis,
) -> UpdateChangeSummary;
```

**Implementation Notes**:
- Compare target-bound aggregate fidelity and exact consequence selectors, not
  source revision text. A newly required component or partial consequence is a
  new user decision even when the revision is a fast-forward.
- Keep summary generation pure and preserve exact target/component identities
  for later acknowledgment planning.

**Acceptance Criteria**:
- [ ] A newly blocked required component is surfaced as a compatibility change.
- [ ] A newly partial optional component requires explicit acknowledgment.
- [ ] Identical compatibility analyses produce a no-change summary.

### Unit 3: Status policy projection
**File**: `crates/cli/src/application.rs`
**Story**: `epic-safe-update-automation-policy-status`

```rust
fn status_update_projection(
    documents: &StatusDocuments,
    scope: &StatusScope,
    targets: &StatusTargets,
    observation: &NativeObservation,
) -> (Vec<OutputEntry>, Vec<Warning>, usize);
```

**Implementation Notes**:
- Render decision reason as a bounded output field and count only actionable
  available updates. `off` remains read-only and does not resolve sources;
  `check` reports candidates without classifying them safe.
- Keep status projection side-effect free and reuse the core classifier so the
  daemon and foreground update service cannot diverge.

**Acceptance Criteria**:
- [ ] Status distinguishes disabled, pinned, blocked, check-only, and safe
      candidates.
- [ ] Status never writes state or native configuration while resolving.
- [ ] Plain and JSON output carry the same typed decision reason.

## Implementation Order

1. `epic-safe-update-automation-policy-contract`
2. `epic-safe-update-automation-policy-compatibility`
3. `epic-safe-update-automation-policy-status`

## Testing

- Core decision-table tests cover every mode/intent/reason combination and
  prove resolver errors remain blocked.
- Compatibility tests compare before/after analyses with required and optional
  component deltas and exact selectors.
- CLI tests prove status output is deterministic, read-only, and identical in
  plain/JSON modes.

## Risks

The primary risk is accidentally treating a source revision change as safe
without re-planning its component graph. The classifier therefore consumes
explicit compatibility and acknowledgment deltas; callers cannot omit them by
using a convenience semver path.

## Other agent review

- Direct-read design only; no peer advisory pass was run because this
  autopilot run is intentionally single-agent.
