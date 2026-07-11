<!-- agile-workflow:rules:start -->
## Agile-Workflow Rules

### Tag semantics

A few tags carry load-bearing routing semantics — get these right:

- **`[refactor]`** — behavior-preserving structural change ONLY. Apply the
  black-box test: would any observable behavior change for a caller of the
  public surface? If yes, this is NOT a refactor — drop the tag and let the
  item route through `feature-design`.
  - Counts as refactor: extract a helper to dedupe, split a god file, rename
    for clarity, remove dead code, inline a one-call abstraction.
  - Does NOT count as refactor (even if it feels "structural"): change an API
    signature, swap a storage backend with different consistency guarantees,
    replace a silent failure with an explicit error, split a function in a
    way that changes call-site contracts, "major rework of X."
- **`[perf]`** — performance work. Routes to `perf-design`.
- **`[research]`** — a grounded research engagement: an *input* that grounds
  other work (a decision, a design, an adoption call), not a shippable
  deliverable. Routes **cross-plugin** to `agentic-research:research-orchestrator`,
  not a design-family skill. The work item carries the **commissioning subset** of
  the engagement registration in a `research_dials:` block (the four scoping fields:
  scope_authority, verification_rigor, intent, output_kind) — **scoping the item IS
  the dispatch act**; the orchestrator reads those dials at kickoff and settles the
  rest at dispatch. A `[research]` item **does not bind to a release** (it is
  an input, not a bundle member) and its verification **gates run inline** in the
  orchestrator (it never reaches `release-deploy`). Routes through `feature-design`
  only as the inert-tag fallback.

All other tags are project-specific (see `.work/CONVENTIONS.md`) and do not
affect skill routing.

### Test integrity

When running, writing, or modifying tests:

- **File real production bugs as backlog items.** When a test failure
  surfaces an actual product bug (not a stale fixture, drifted assertion,
  or broken mock), park it via `/agile-workflow:park` instead of silently
  fixing it inline mid-test-pass. The backlog item is the audit trail.
- **Fix bad tests in-session.** Stale fixtures, drifted assertions, broken
  mocks, and outdated snapshots are test debt, not product bugs. Repair
  them as you go so the suite stays meaningful.
- **Then drain small backlog bugs with a full pass.** Once tests are
  green again, if a parked production bug is small enough for a single
  stride, pick it up immediately as `/agile-workflow:scope` → design →
  implement. Larger bugs stay in backlog for prioritization.
- **NEVER game a test to make it pass.** A failing test that documents
  *why* it fails — an inline comment naming the bug, a `skip` linked to a
  backlog id, an `xfail` with a reason — is more honest than a green test
  that lies. No `expect(true).toBe(true)`, no asserting on whatever the
  code happens to return, no deleting a test as "flaky" without
  root-causing first.

Cross-model advisory review: explicit user/project review instructions
override agile-workflow defaults. When peeragent is available with a different
model class, large/risky autopilot design decisions may use one advisory pass;
small/low-risk work skips it. Autopilot also runs a final peer-review loop
before reporting completion and fixes or files accepted findings first.
Same-model peers fall back to local sub-agents instead. Claude Opus peeragent
calls can take 10 to 30 minutes on large reviews; no return after a few minutes
is not evidence that the call has hung.

Broad entry points:
`/agile-workflow:ideate`, `/agile-workflow:epicize`,
autopilot goals such as "Use agile-workflow autopilot to drain --all",
and `/agile-workflow:release-deploy`.
<!-- agile-workflow:rules:end -->
