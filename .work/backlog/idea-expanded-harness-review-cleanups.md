---
id: idea-expanded-harness-review-cleanups
created: 2026-07-15
updated: 2026-07-15
tags: [cleanup]
---

Follow up on three non-blocking observations from the final expanded-harness-support review:

- `crates/harnesses/src/adapters/factory.rs`: document and test that marketplace observation intentionally remains `Indeterminate(UnsupportedShape)` because the attested Factory marketplace list has no safely parseable shape; plugin observation remains available.
- `crates/test-support/src/bounded_process.rs`: make the reported error deterministic when process deadline and output limit fail together by preserving the first failure rather than allowing the output drain to replace it.
- `crates/harnesses/src/adapters/gemini.rs`: consider requiring agreement between the status prefix and the `Connected` suffix instead of allowing the unhealthy prefix to win for a contradictory hypothetical line.
