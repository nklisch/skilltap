---
id: epic-harness-observation-adoption-runtime-adversarial-fixtures
kind: story
stage: review
tags: [testing,infra]
parent: epic-harness-observation-adoption-runtime
depends_on: []
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Build Native Observation Adversarial Fixtures

Extend test support with neutral fake-native process and external-tree fixtures.
Cover exact argv/environment, non-zero exit, hang, stdout/stderr/both-stream
flooding, descendants that retain pipes, a `setsid`-escaped descendant that
leaves the original process group while retaining pipes, deterministic process barriers,
deep/wide/oversized trees, live/dangling links, FIFO/socket entries, permission
errors, and deterministic fault-injected file/tree replacement and permission
races rather than timing or chmod assumptions. Fixtures expose no
harness interpretation and do not create a test-support dependency from core;
cfg-specific Unix behavior must execute natively on Linux and macOS.

## Implementation notes

- Files changed: `crates/test-support/{Cargo.toml,build.rs}`,
  `crates/test-support/fixtures/escaped_pipe_holder.c`,
  `crates/test-support/src/{lib.rs,barrier.rs,native_process.rs,external_tree.rs}`.
- Tests added: ten new focused tests cover byte-exact argv/environment/cwd
  capture and safe Debug output; non-zero and independent stdout/stderr flood
  modes; deterministic start barriers; child, descendant, and `setsid`-escaped
  pipe holders; deep, wide, sparse-oversized, symlink, FIFO, and socket trees;
  injected permission failures; and barrier-controlled file/tree replacements.
- Discrepancies from design: permission cases use explicit injected
  `PermissionDenied` faults instead of chmod assumptions, and escaped descendant
  support embeds a tiny build-time POSIX helper so Linux and macOS do not depend
  on a platform `setsid` executable.
- Adjacent issues parked: none.
