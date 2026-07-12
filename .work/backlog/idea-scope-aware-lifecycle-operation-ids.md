---
id: idea-scope-aware-lifecycle-operation-ids
created: 2026-07-12
updated: 2026-07-12
tags: [correctness]
---

The native lifecycle operation identifier currently hashes the target and
resource id but not the concrete scope. A project-scoped and global resource
with the same id therefore produce duplicate operation ids when a command
uses `--all-scopes`; `plugin remove ... --all-scopes` fails closed with
`operation_plan_invalid` before mutation. Include the scope in the operation
identity and retain the all-scopes regression coverage in
`crates/cli/tests/compiled_binary.rs`.
