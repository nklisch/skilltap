---
id: epic-real-harness-recovery-state-diagnostics-output-contract
kind: story
stage: implementing
tags: [correctness, testing, documentation]
parent: epic-real-harness-recovery-state-diagnostics
depends_on:
  - epic-real-harness-recovery-state-diagnostics-dual-native-lifecycle
  - epic-real-harness-recovery-state-diagnostics-update-eligibility
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-12
updated: 2026-07-12
---

# Align help and diagnostic aggregation

## Scope

Make exact plugin removal grammar discoverable, deduplicate exact next actions
at outcome merge boundaries, and carry repaired adapter diagnostics through
post-mutation output without replacing them with generic errors. Roll SPEC and
UX examples forward with the executable contract.

## Acceptance

- Plugin removal help and parsing require `PLUGIN@MARKETPLACE` consistently in
  root/group/leaf, plain/JSON, SPEC, and UX surfaces.
- Exact duplicate actions render once in first-seen order; materially distinct
  commands or explanations remain.
- Ordered normalization is idempotent and does not alter result or exit class.
- Native post-mutation failures retain the typed boundary reason and one
  actionable recovery command; healthy final observation completes normally.
- Multi-target/all-scope compiled-binary tests prove plain/JSON parity.

## Implementation

- Plugin removal now parses and advertises only exact
  `PLUGIN@MARKETPLACE` selectors across leaf help, safe parse failures, SPEC,
  and UX.
- Outcome builders and both renderers normalize exact duplicate recovery
  actions without collapsing distinct commands. A top-level action is the
  canonical rendered copy when an error also carries the same action.
- Typed native detection and postcondition failures retain their stable reason
  and actionable status command after reconciliation or mutation.
- Isolated compiled tests cover typed postcondition plain/JSON parity and a
  two-target, two-scope diagnostic matrix without touching user state.

## Verification

- `cargo test -p skilltap --lib`
- `cargo test -p skilltap --test native_postconditions native_postcondition_diagnostic_has_plain_and_json_parity -- --exact`
- `cargo test -p skilltap --test native_postconditions multi_target_all_scope_diagnostics_have_plain_and_json_parity -- --exact`

## Review checkpoint (2026-07-12)

The independently committed grammar and normalization portions are approved
at fresh-context `standard` depth:

- `plugin remove` consistently requires `PLUGIN@MARKETPLACE` in parsing, leaf
  help, safe plain/JSON invalid-input recovery, SPEC, and UX.
- Exact next actions are deduplicated in first-seen order at builders and both
  renderers; different commands/summaries remain distinct, normalization is
  idempotent, and rendering does not mutate the source outcome or result class.
- Focused command, outcome, renderer, release-help, and compiled invalid-input
  tests pass at commits `6c657f0` and `ea61cb1`.

No blocker or important finding applies to those portions. Final story review
remains pending only on typed native post-mutation output and its multi-target
plain/JSON integration evidence.

## Review findings (2026-07-12)

- **Blocker — the committed focused suite contradicts the canonical action
  contract.** `Outcome::normalize_next_actions` correctly retains one exact
  recovery action at the top level and removes the identical nested error copy,
  but `native_postcondition_failures_are_typed_and_never_journal_success` still
  indexes the removed nested action. The full `native_postconditions` target
  therefore fails 1 of 5 tests. Align the regression with the approved one-copy
  contract and retain its typed error/state-safety assertions. Tracked by
  `epic-real-harness-recovery-state-diagnostics-output-test-parity`.

## Review (2026-07-12)

**Verdict**: Request changes

**Blockers**: focused compiled suite is red
(`epic-real-harness-recovery-state-diagnostics-output-test-parity`)

**Important**: none

**Nits**: none

**Notes**: Fresh-context deep review at caller-selected `standard` weight,
escalated for the public plain/JSON contract. Exact global/error duplicate
normalization, first-seen ordering, distinct-command retention, result/exit
preservation, typed post-mutation output, and multi-target/all-scope parity all
pass focused isolated checks. Approval is withheld solely because the complete
focused test target is red on its stale nested-action assertion.
