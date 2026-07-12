---
id: story-plan-nonempty-exit-class
kind: story
stage: implementing
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
