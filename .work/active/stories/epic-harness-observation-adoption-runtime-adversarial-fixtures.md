---
id: epic-harness-observation-adoption-runtime-adversarial-fixtures
kind: story
stage: done
tags: [testing,infra]
parent: epic-harness-observation-adoption-runtime
depends_on: []
release_binding: 3.0.0
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
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

## Review

- Initial review found parallel `ETXTBSY` failures and missing direct hang
  liveness/reap coverage.
- Correction `c17805b` moved fake executables to build-time stable inodes and
  gives each fixture a hard link plus non-executable `$0`-keyed behavior file;
  it adds the hang readiness/kill/reap assertion.
- Fresh re-review passed 20/20 churn stress iterations, 30/30 normal parallel
  package runs, escaped `setsid` retention, hang cleanup, and Linux/macOS
  portability/injection checks.

## Review correction

- Root cause: fake executables were written directly at their final path and an
  immediate parallel `exec` can intermittently receive `ETXTBSY` on the test
  filesystem even after the convenience write returns.
- The build script now publishes one generic fake-native executable and escaped
  pipe-holder helper under `OUT_DIR`, before tests start. Each fixture creates
  only unique hard links to those stable executable inodes and a non-executable
  behavior file resolved from `$0`; keeping the fixture root on the same
  filesystem also means canonical executable resolution preserves that unique
  hard-link path. Raw `command()` and `executable()` use the same path as
  product-runner tests, with no hidden retry, environment, or argv.
- Added a parallel stress regression (eight workers creating and executing 32
  fixtures each) that failed immediately before the correction, plus a focused
  hang regression that observes readiness, proves liveness, kills, reaps, and
  verifies that the fixture PID no longer exists. Hang uses `exec sleep`, so it
  cannot leave a shell child orphan.
- Verification: the stress regression passed 20 consecutive iterations and the
  normal parallel package suite passed 30 consecutive iterations.
