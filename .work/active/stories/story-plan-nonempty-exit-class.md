---
id: story-plan-nonempty-exit-class
kind: story
stage: done
parent: null
depends_on: []
release_binding: 3.0.0
created: 2026-07-12
updated: 2026-07-12
tags: [correctness]
---

# Return attention for populated reconciliation plans

The populated reconciliation `plan` command now renders operations but returns
`result=completed` and exit code 0. The documented CLI contract requires a
non-empty plan to return attention/exit code 2 while remaining side-effect
free. Preserve the existing compiled regressions and correct the result class
without changing the operation details.

## Implementation scope

When `plan` emits one or more operations or findings, return the documented
attention result/exit code 2 while remaining side-effect free. Preserve
completed/no-change for an empty plan and retain the compiled regressions.

## Source

Promoted from `idea-plan-nonempty-exit-class` after the final workspace test
pass exposed the production regression.

## Implementation Notes

- Count planned operations before classifying the result. A populated,
  side-effect-free plan now returns `attention_required` (exit 2), including
  no-op and repair operations that still require caller inspection.
- Empty plans remain `completed` (exit 0); observation failures and existing
  errors/warnings retain their higher-severity result classes.
- Existing compiled regressions cover populated, healthy, and drifted plans;
  all pass without filesystem mutation.
- Verification: `cargo fmt --all -- --check`; `cargo test -p skilltap
  --test compiled_binary --offline`; `cargo test --workspace --all-targets
  --offline`; `cargo clippy --workspace --all-targets --offline -- -D
  warnings`.

## Review

Approved. The populated-plan result classification now derives from the
rendered operation count, so any non-empty plan returns `attention_required`
and exit code 2 while remaining side-effect free. Empty plans remain
`completed` with exit code 0, and existing observation/error precedence is
preserved. The adjacent output change removes the stale unavailable-adapter
warning and next action without changing the plan payload. The compiled
populated, healthy, and drifted-plan regressions, the full workspace tests,
format check, and warnings-denied Clippy all pass.
