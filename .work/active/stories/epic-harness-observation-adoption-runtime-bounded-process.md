---
id: epic-harness-observation-adoption-runtime-bounded-process
kind: story
stage: done
tags: [infra,correctness]
parent: epic-harness-observation-adoption-runtime
depends_on: [epic-harness-observation-adoption-runtime-contracts-limits, epic-harness-observation-adoption-runtime-adversarial-fixtures, epic-harness-observation-adoption-runtime-executable-resolution]
release_binding: null
research_refs: []
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Run Bounded Native Processes

Replace unbounded observation command execution with direct `OsString` argv,
null stdin, explicit cleared environment, optional canonical cwd, and the
resolved absolute executable. Concurrently drain stdout/stderr while enforcing
per-stream and combined caps through nonblocking owned readers. On timeout or
overflow, terminate the dedicated Unix process group and always reap. Apply a
hard post-kill drain deadline and close parent read descriptors so even a
`setsid`-escaped descendant retaining pipe handles cannot block completion.
Return non-zero exit as a bounded result, revalidate executable identity just
before spawn, and keep all errors/output Debug-safe in native Linux and macOS
behavior suites.

## Implementation

- Added `SystemNativeProcessRunner` with direct absolute executable invocation,
  explicit cleared environment, null stdin, optional cwd, immediate identity
  revalidation, and a dedicated process group.
- Parent-owned nonblocking stdout/stderr readers drain concurrently while
  enforcing per-stream and combined caps. Deadline and overflow paths kill the
  group, fall back to direct child termination if necessary, reap the child,
  and close parent descriptors after a hard post-kill drain window for escaped
  descendants.
- Added direct tests for nonzero status, literal args, explicit environment and
  cwd forwarding, stream flood limits, hang timeout/reap, and escaped-pipe
  completion. No native payloads enter Debug or error text.
- Process drain/termination failures use the distinct closed runtime error
  categories introduced by the contracts story.

## Review correction

- The review identified that group-only signaling could leave a `setsid` child
  alive and that the final blocking `wait` could hang after the drain deadline.
  Termination now always attempts the direct child handle after group signaling,
  converts `try_wait` failures into cleanup state, and uses bounded polling for
  reaping with no unbounded wait fallback.
- The escaped-descendant test now gates helper startup, waits for confirmed
  pipe retention, releases the helper after the runner returns, and waits for
  its exit marker and process disappearance before dropping the fixture root.
  The helper's fork parent waits for the post-`setsid` readiness marker, so the
  test proves the escaped process-group case rather than racing it.

## Verification

- Focused bounded-process tests pass 6/6 with warnings-denied core Clippy.
- Full workspace format/check/Clippy/tests pass: 209 core tests, 3 foundation
  integrations, 3 storage integrations, 15 fixture tests, and 6 compiled
  binary tests.
- Workspace rustdoc, release build, and compiled-binary verification pass.

## Review

- Independent deep review approved the direct-child fallback, bounded reap,
  `try_wait` cleanup, post-`setsid` startup handshake, and exact escaped-helper
  disappearance check. Repeated escaped-process runs left no helper processes.
- The review also accepted the focused direct-fallback and reap-deadline
  regressions as sufficient coverage; syscall-fault injection remains outside
  this story's required surface.
