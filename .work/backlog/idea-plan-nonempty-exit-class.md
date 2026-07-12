---
id: idea-plan-nonempty-exit-class
created: 2026-07-12
updated: 2026-07-12
tags: [correctness]
---

The populated reconciliation `plan` command now renders operations but returns
`result=completed` and exit code 0. The documented CLI contract requires a
non-empty plan to return attention/exit code 2 while remaining side-effect
free. Preserve the existing compiled regressions and correct the result class
without changing the operation details.
