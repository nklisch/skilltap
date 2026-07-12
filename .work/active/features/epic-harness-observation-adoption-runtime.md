---
id: epic-harness-observation-adoption-runtime
kind: feature
stage: done
tags: [infra]
parent: epic-harness-observation-adoption
depends_on: [epic-harness-observation-adoption-contracts]
release_binding: 3.0.0
research_refs:
  - .research/analysis/briefs/current-agent-extension-standards.md
research_origin: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-12
---

# Safe Native Observation Runtime

Provide the bounded read-only substrate for native adapters: canonical PATH or
absolute executable resolution with file identity; direct-argument processes
with deadline, output limits, null stdin, kill/reap, explicit environment, and
safe status; strict one-document UTF-8 JSON boundaries with duplicate/trailing/
depth/size rejection; `CODEX_HOME`-aware paths; and bounded descriptor-relative
external directory observation that reports links and rejects non-regular or
raced entries without following them. Include adversarial process/filesystem
fixtures. This feature performs no harness-specific interpretation.

## Design

### Boundary and limits

The runtime is a set of harness-neutral adapters behind core observation
contracts. Every external boundary receives explicit non-zero limits and
returns a closed error category whose Debug/Display forms contain no argv,
environment values, native output, file bytes, parser excerpts, or raw paths.
Limits have hard compile-time ceilings and checked cross-field relationships;
callers cannot request pathological deadlines, recursion, allocation sizes, or
counter ranges. They cover process deadline, stdout/stderr and combined output, JSON bytes
and nesting depth, tree depth and entries, per-file bytes, total tree bytes,
and symlink-target bytes.

The runtime accepts a configured binary plus an explicit PATH value, resolves
one canonical executable identity, and revalidates that identity immediately
before every spawn. Empty PATH components are invalid rather than implicitly
meaning the current directory. Final executable symlinks may resolve to a
canonical regular executable so Homebrew and version-manager installations
work. Revalidation narrows the race window but is not described as proving the
executed inode; an fd-based execution mechanism would require a later explicit
cross-platform design.

### Process and structured-data safety

Native commands use direct argument vectors, null stdin, an explicit cleared
environment, optional canonical working directory, and the previously resolved
absolute executable. Stdout and stderr are drained concurrently while limits
are enforced. Timeout or overflow terminates the dedicated Unix process group
and always reaps the child. Pipes are nonblocking and owned by a bounded
poll/select/kqueue-style loop; after termination a short hard drain deadline
closes parent read descriptors, so a descendant that escapes the process group
with `setsid` cannot keep the command waiting forever. Non-zero native exit is
a bounded result, not an infrastructure error.

JSON decoding first applies a byte cap and UTF-8 boundary, then accepts exactly
one document. A custom recursive visitor rejects duplicate keys at every depth,
trailing documents/garbage, and depth overflow before typed deserialization.
Errors expose only category and configured limits.

### External paths and trees

Codex-native paths honor a non-empty normalized absolute `CODEX_HOME`; absent or
empty falls back to `$HOME/.codex`. XDG continues to control only skilltap state,
and the canonical global instruction remains `~/AGENTS.md`. Resolution creates
nothing.

External harness trees are observed separately from skilltap-managed artifact
trees. Traversal is descriptor-relative and no-follow, deterministic, and
bounded while walking. Regular files are read only after identity checks;
directories are traversed; symlinks are reported with bounded opaque targets
but never followed. External tree snapshots are non-serializable with custom
redacted Debug; raw target bytes are accessible only inside the owning adapter
and never enter findings, errors, state, or output. FIFO, socket, device, non-UTF-8, raced, over-depth,
over-entry, and over-byte entries fail with safe typed context.

### Pre-mortem

- **A child keeps pipes open after timeout.** Put the native process in its own
  group, drain both streams nonblockingly, kill the group, reap, and close
  parent readers after a hard post-kill drain deadline even if an escaped
  descendant retains the write ends.
- **A large output is buffered before the cap.** Enforce per-stream and total
  caps during reads rather than after `wait_with_output`.
- **JSON silently accepts duplicate keys.** Parse through a duplicate-aware
  recursive visitor before typed decoding; do not rely on `serde_json::Value`.
- **A file changes between metadata and read/spawn.** Bind identities to opened
  descriptors where available and revalidate at the last boundary; report
  replacement rather than continuing.
- **Tree traversal follows a link or blocks on a FIFO.** Use descriptor-relative
  no-follow opens, classify before reading, and reject special files.
- **Environment inheritance leaks secrets or changes behavior.** Clear the
  child environment and pass an explicit allowlist supplied by the adapter.
- **Linux-only behavior lands accidentally.** Keep errno/process-group and
  descriptor primitives portable across Linux and macOS and run native
  behavior suites on both platforms.

## Implementation units

1. `epic-harness-observation-adoption-runtime-contracts-limits` — define
   bounded runtime requests, statuses, limits, ports, and safe errors — depends
   on `[]`.
2. `epic-harness-observation-adoption-runtime-adversarial-fixtures` — add
   process and external-tree fixtures for timeout, overflow, descendants,
   special files, bounds, and races — depends on `[]`.
3. `epic-harness-observation-adoption-runtime-executable-resolution` — resolve
   configured binaries to canonical executable identities and revalidate them
   — depends on `[runtime-contracts-limits, runtime-adversarial-fixtures]`.
4. `epic-harness-observation-adoption-runtime-bounded-process` — execute direct
   bounded native processes with process-group termination and reaping —
   depends on `[runtime-contracts-limits, runtime-adversarial-fixtures,
   runtime-executable-resolution]`.
5. `epic-harness-observation-adoption-runtime-strict-json` — implement the
   bounded duplicate-aware one-document JSON boundary — depends on
   `[runtime-contracts-limits]`.
6. `epic-harness-observation-adoption-runtime-codex-home` — add safe
   `CODEX_HOME` resolution without moving `~/AGENTS.md` — depends on
   `[runtime-contracts-limits]`.
7. `epic-harness-observation-adoption-runtime-external-tree` — implement
   bounded descriptor-relative no-follow external tree observation — depends
   on `[runtime-contracts-limits, runtime-adversarial-fixtures]`.
8. `epic-harness-observation-adoption-runtime-integration` — verify the whole
   resolve/run/decode/path/tree pipeline, determinism, safety, and platform
   gates — depends on all five concrete runtime adapters.

## Acceptance criteria

- Executable resolution is deterministic, scope-free, canonical, and bound to
  file identity; unsafe PATH and replacement cases fail explicitly.
- Process fixtures include a `setsid`-escaped descendant retaining pipe handles;
  processes receive direct argv, null stdin, exact environment and cwd, enforce
  deadline and all output caps during concurrent reads, terminate descendants,
  and always reap.
- JSON rejects over-limit bytes/depth, invalid UTF-8, duplicate keys, trailing
  documents, and trailing garbage without echoing source bytes.
- Codex paths honor `CODEX_HOME` fallback/override rules, create nothing, and do
  not relocate global `~/AGENTS.md`.
- External trees are deterministic and bounded, report but never follow links,
  reject special/non-UTF-8/raced entries, and never reuse managed write APIs.
- Tree snapshots cannot serialize raw file or link-target bytes, use redacted
  Debug, and keep opaque targets inside the owning adapter boundary.
- Adversarial fixtures cover boundary minus/at/plus one, both-pipe pressure,
  descendant pipe holders, executable replacement, tree swaps, and secret
  canaries.
- Full locked format/check/Clippy/test/rustdoc, release/compiled-binary, and
  native Linux and macOS runtime behavior gates pass.

## Implementation

- Completed all eight runtime stories: bounded contracts, adversarial native
  and filesystem fixtures, executable identity resolution, direct bounded
  process execution, strict JSON, Codex-home-aware paths, descriptor-relative
  external trees, and end-to-end composition tests.
- The runtime remains harness-neutral and read-only. Native process cleanup
  uses group plus direct-child termination with bounded reaping; external trees
  never follow links or reuse managed write APIs; all sensitive payloads stay
  behind redacted Debug/error boundaries.

## Verification

- 211 core tests, CLI/integration suites, 15 fixture tests, workspace Clippy,
  rustdoc, release build, and compiled-binary verification pass under the
  locked workspace. The runtime composition suite adds three repeatable,
  read-only resolver/process/JSON/path/tree checks.
- Linux behavior is exercised in CI and the fixture contracts are cfg-gated for
  native macOS execution; no platform-specific production fallback was added.

## Review

- Aggregate feature review approved after verifying all eight child stories,
  their dependency order, runtime boundary contracts, and the locked workspace
  ladder. No production writes, marketplace discovery, raw native payload
  leakage, or unbounded process/tree operations were introduced.
- Fresh macOS execution remains CI-gated rather than available in this Linux
  workspace; the Unix implementations and cfg-specific fixture suites are
  covered by the existing native behavior job.
