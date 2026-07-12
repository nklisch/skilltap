---
id: gate-security-git-argument-delimiters
kind: story
stage: done
tags: [security]
parent: null
depends_on: []
release_binding: 3.0.0
gate_origin: security
created: 2026-07-12
updated: 2026-07-12
---

# Delimit and validate external Git arguments

## Severity

Medium

## Domain

Input validation and command boundaries

## Location

- `crates/harnesses/src/update_resolution.rs:61-72`
- `crates/cli/src/application.rs:4163-4168,4217-4227`
- `crates/harnesses/src/lifecycle.rs:278-282,316-320`

## Evidence

External locator and requested-revision strings are passed positionally to Git
and native lifecycle commands without `--` delimiters; boundary validation does
not reject option-like values beginning with `-`.

## Remediation direction

Use `--` delimiters wherever supported, reject option-like locators and
revisions at the input boundary, and add adversarial direct-argv tests.

## Autopilot implementation note

The remediation and affected boundaries are explicit. Preserve native
argument-vector execution and add validation/tests in the named adapters.

## Implementation

- Rejected leading-dash source locators and requested revisions both at the
  explicit skill-install boundary and inside the Git source resolver.
- Added Git `--` delimiters before repository and fetched refspec values while
  preserving valid SCP-style locators such as `git@example.test:team/repo.git`.
- Rejected leading-dash native lifecycle names and marketplace sources before
  constructing Codex or Claude argument vectors; native harness command syntax
  does not document an end-of-options delimiter for these positional values.
- Added direct-argv and adversarial validation tests for both adapters.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p skilltap-harnesses --offline` (10 passed)
- `cargo test -p skilltap --offline` compiled and unit tests passed; two
  unrelated compiled-binary tests remain failing in concurrent reconciliation
  and daemon work (`native_mutations_keep_project_and_all_scope_boundaries`,
  `safe_update_cycle_reports_changed_git_revision_and_records_daemon_result`).

## Review (2026-07-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Standard substrate review with deep command-boundary/security
lenses. Git locators and revisions are rejected when option-like and are
delimited in `ls-remote`, clone, and fetch argument vectors; native lifecycle
names and sources are similarly validated before vector construction. SCP-style
locators remain valid. Harness and CLI focused tests pass; the noted compiled
binary failures are unrelated concurrent reconciliation/daemon stories.
