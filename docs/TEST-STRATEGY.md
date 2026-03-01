# Test Strategy — Coverage Gaps and Improvement Plan

## Current State

437 tests across 40 files. Strong coverage of schemas, security pattern detection, git operations, and CLI command surfaces. Test infrastructure is solid: fixture factories, temp dir lifecycle, subprocess spawning with env isolation.

## Coverage Gaps

Three structural gaps drive the plan:

**1. Core logic tested only through CLI subprocesses.** `remove.ts`, `update.ts`, and `symlink.ts` have no unit tests. Bugs surface at the wrong level — slow, hard to pinpoint, no branch coverage.

**2. Agent adapter `invoke()` never tested.** `detect()` is covered; `invoke()` is not. The `createOllamaAdapter` and `createCustomAdapter` factories have zero coverage.

**3. Agent mode output format has no stability guarantee.** Agents parse this output. An accidental format change would silently break agent integrations.

Secondary gaps: edge cases for error recovery, `installed.json` schema backward compatibility, and no performance regression detection.

---

## Phase T1 — Core Unit Tests

*Adds direct unit coverage for `remove.ts`, `update.ts`, and `symlink.ts` — currently only reachable through slow CLI subprocess tests.*

### T1.1 — `symlink.ts` unit tests

New file: `packages/core/src/symlink.test.ts`

Test cases:
- `createAgentSymlinks` — creates symlink at correct agent path for global scope
- `createAgentSymlinks` — creates symlink at correct agent path for project scope
- `createAgentSymlinks` — creates parent directories if missing
- `createAgentSymlinks` — returns `UserError` for unknown agent identifier
- `createAgentSymlinks` — creates symlinks for multiple agents in one call
- `removeAgentSymlinks` — removes existing symlinks, silently skips missing ones
- `removeAgentSymlinks` — treats `linked` scope as `global` for path resolution

Use `makeTmpDir()` as the base; override `SKILLTAP_HOME` so `globalBase()` points to temp dir. Verify symlink existence with `Bun.file(...).exists()` after each operation.

### T1.2 — `remove.ts` unit tests

New file: `packages/core/src/remove.test.ts`

Setup: install a skill first (using `installSkill` with `skipScan: true`) or write `installed.json` directly using `saveInstalled`.

Test cases:
- Removes global skill: directory deleted, record gone from `installed.json`
- Removes project skill: correct install path used
- Removes linked skill: symlink at `record.path` deleted (not computed install path)
- Returns `UserError` when skill name not found
- Returns `UserError` when name matches but scope filter doesn't
- Cache cleanup: removes cache dir when last skill from that repo is removed
- Cache preserved: keeps cache dir when other skills from same repo remain
- Agent symlinks removed as part of removal

### T1.3 — `update.ts` unit tests

New file: `packages/core/src/update.test.ts`

These require a real git repo that can be fetched. Use `createStandaloneSkillRepo()` to create the initial install, add a second commit to the source repo, then run `updateSkill`.

Test cases:
- Reports `upToDate` when local SHA equals remote SHA (no new commits)
- Returns `updated` after applying a clean new commit
- `onProgress` callback fires with correct status values in sequence
- `onDiff` callback receives correct `DiffStat` and SHAs
- `record.sha` and `record.updatedAt` written to `installed.json` after update
- Skips linked skills, fires `onProgress` with `"linked"`
- `strict: true` — skips skill when diff has security warnings
- `onConfirm` returning `false` adds skill to `skipped`
- `name` filter — only updates the named skill
- Returns `UserError` when named skill not installed
- Multi-skill: re-copies skill subdirectory from cache after pull

**Exit criteria:** `packages/core/src/` has 3 new test files. `bun test packages/core/` covers all branches of remove, update, and symlink without spawning subprocesses.

---

## Phase T2 — Agent Adapter Invoke Tests

*Adds invoke() coverage using a mock agent binary. Establishes the `createMockAgentBinary` test utility.*

### T2.1 — Mock agent binary helper

New export in `packages/test-utils/src/agents.ts`:

```ts
export async function createMockAgentBinary(
  response: string,
  exitCode = 0,
): Promise<{ binaryPath: string; cleanup: () => Promise<void> }>
```

Implementation: write a small shell script to a temp dir (`#!/bin/sh\necho '<response>'\nexit <code>`), `chmod +x`, return path. Cleanup removes the temp dir.

This is a shell script, not a compiled binary — works portably without build steps.

### T2.2 — `createCliAdapter` invoke tests

New file: `packages/core/src/agents/__tests__/factory.test.ts`

Test cases:
- `invoke()` with mock binary returning valid JSON object → `ok({ score, reason })`
- `invoke()` with mock binary returning JSON in code block → extracted correctly
- `invoke()` with mock binary returning unparseable output → `ok({ score: 0, reason: "..." })`
- `invoke()` with mock binary exiting non-zero → `err(ScanError)`
- `invoke()` with non-existent binary path → `err(ScanError)` (no crash)
- `detect()` with binary on PATH → `true` (use the mock binary directory prepended to PATH)
- `detect()` with binary not on PATH → `false`

### T2.3 — `createCustomAdapter` tests

New file: `packages/core/src/agents/__tests__/custom.test.ts`

Test cases:
- `detect()` returns `true` when binary file exists
- `detect()` returns `false` when binary file missing
- `invoke()` with mock binary returning valid response → `ok(parsed)`
- `invoke()` with mock binary returning garbled output → `ok({ score: 0, ... })`
- `invoke()` failure → `err(ScanError)` with message

### T2.4 — `createOllamaAdapter` tests

New file: `packages/core/src/agents/__tests__/ollama.test.ts`

Ollama calls `which ollama` and `ollama list` in `detect()`, then `ollama run` in `invoke()`. Test by creating a mock `ollama` binary.

Test cases:
- `detect()` — mock `ollama` binary present, `ollama list` returns two lines → `true`
- `detect()` — mock `ollama` binary present, `ollama list` returns one line (header only) → `false`
- `detect()` — mock binary absent → `false`
- `invoke()` — mock binary echoes valid JSON response → `ok(parsed)`
- `invoke()` — uses `"llama3"` as default model when factory called with empty string

**Exit criteria:** Agent `invoke()` is tested end-to-end for all adapter types. `createMockAgentBinary` is exported from `@skilltap/test-utils`.

---

## Phase T3 — Agent Mode CLI E2E

*Validates the two-branch pattern at the CLI level: agent mode must produce clean plain-text output with no ANSI, correct exit codes, and the security block directive.*

### T3.1 — Extend test utilities

Add to `packages/test-utils/src/fixtures.ts`:

```ts
export async function createMaliciousSkillRepo(): Promise<...>  // already exists
export async function createTapWithSkills(skills: {...}[]): Promise<...>  // new
```

Add to `packages/cli/src/commands/` test helpers (or inline in test):

```ts
async function writeAgentModeConfig(configDir: string, overrides = {}): Promise<void>
```

### T3.2 — Agent mode install tests

New file: `packages/cli/src/commands/install.agent-mode.test.ts`

Each test writes an agent-mode config to an isolated `configDir`, then runs `install` as a subprocess.

Test cases:
- Successful install in agent mode: exit 0, no ANSI codes in stdout, contains skill name
- `--skip-scan` blocked in agent mode: exit 1, error message about agent mode
- Security issue in agent mode: exit 1, stdout contains `SECURITY ISSUE FOUND — INSTALLATION BLOCKED` and `DO NOT install`
- Already installed in agent mode: exit 0 (or exit 1 per SPEC), `agentSkip` output format
- Multiple skills in multi-skill repo: auto-selects all (no prompt), installs all, exit 0

### T3.3 — Agent mode update tests

New file: `packages/cli/src/commands/update.agent-mode.test.ts`

Test cases:
- Up-to-date skill in agent mode: exit 0, `agentUpToDate` output
- Update available: exit 0, applied without confirmation, `agentSuccess` output
- Update with security warnings in diff: exit 1, `agentSecurityBlock` output
- Named skill update: `update <name>` in agent mode

**Exit criteria:** Agent mode CLI behavior is validated end-to-end. Subprocess tests confirm no ANSI leakage and correct exit codes.

---

## Phase T4 — Output Stability and Contract Tests

*Catches accidental format regressions. Agent mode output and `installed.json` structure are external contracts.*

### T4.1 — Agent output snapshot tests

Extend `packages/cli/src/ui/agent-out.test.ts` with snapshot assertions.

For each of the five output functions (`agentSuccess`, `agentError`, `agentSkip`, `agentUpToDate`, `agentSecurityBlock`), add a test that captures output to a string and calls `expect(output).toMatchSnapshot()`.

Snapshots committed to the repo. A CI run that changes these requires explicit snapshot update (`bun test --update-snapshots`), making format changes deliberate.

Key invariants to assert even without snapshots:
- Output contains no ANSI escape codes (`/\x1b\[/` regex)
- `agentSecurityBlock` contains the literal strings `SECURITY ISSUE FOUND` and `DO NOT install`
- `agentError` output ends with newline

### T4.2 — `installed.json` backward compatibility

New file: `packages/core/src/schemas/installed.compat.test.ts`

Write out `installed.json` fixtures representing older schema shapes (e.g., missing `description`, missing `sha`, missing `also`), then assert that `loadInstalled()` parses them successfully without error.

One fixture per known schema evolution. These are static JSON fixtures committed to `packages/test-utils/fixtures/compat/`.

This test fails if a schema change breaks existing user installations.

### T4.3 — Config TOML round-trip contract

Extend `packages/core/src/config.test.ts`:

- Load config with extra unknown keys → unknown keys silently ignored, known keys preserved
- Save config with all optional fields → reload produces same values
- Config with partial sections (only `[security]` block, no `[agent-mode]`) → loads with correct defaults for missing sections

**Exit criteria:** Format regressions in agent output or schema changes that break existing installations are caught in CI before merge.

---

## Phase T5 — Edge Cases and Error Recovery

*Tests failure modes and boundary conditions that the happy-path tests don't reach.*

### T5.1 — Dangling symlink handling

Extend `packages/core/src/symlink.test.ts`:

- `removeAgentSymlinks` with a dangling symlink (target deleted, link exists): succeeds without error
- `createAgentSymlinks` when symlink already exists at path: behavior per SPEC (error or overwrite — document whichever is correct)

Extend `packages/cli/src/commands/info.test.ts`:

- `info` on a linked skill whose source path no longer exists: exit 0 (or 1), meaningful error message rather than crash

### T5.2 — Idempotency tests

Extend `packages/core/src/install.test.ts`:

- Install same skill twice: second call returns `UserError` "already installed", `installed.json` has exactly one record
- Install same skill with `--force` (if SPEC supports it) or remove then reinstall: clean state

### T5.3 — Concurrent state integrity (sequential simulation)

Not truly concurrent, but simulate partial writes:

- Write a truncated/corrupt `installed.json` → `loadInstalled` returns error or empty state, does not crash
- `saveInstalled` with very large skills array → succeeds, file is valid TOML/JSON

### T5.4 — Network error simulation

Extend `packages/core/src/git.test.ts`:

- `clone` with invalid URL → returns `GitError` with useful message, no tmp dir left behind
- `fetch` when remote is unreachable → `GitError`, skill record unchanged

**Exit criteria:** The error paths in core are exercised and verified to return appropriate `Result<_, E>` values rather than throwing or corrupting state.

---

## Phase T6 — Performance Benchmarks

*Optional, addable at any time. Uses Bun's built-in `bench()`. Not part of `bun test` — run separately.*

New file: `packages/core/src/benchmarks/scan.bench.ts`

```ts
import { bench, run } from "bun:test";
```

Benchmarks:
- `scanStatic()` on a directory with 500 SKILL.md files: must complete < 5s
- `chunkSkillDir()` on a 10,000 line SKILL.md: must complete < 500ms
- `scanDiff()` on a 1MB diff output: must complete < 100ms
- Config `loadInstalled()` with 100 skill records: must complete < 50ms

These benchmarks serve as regression detectors, not strict gates. Add a `bench` script to root `package.json`:

```json
"bench": "bun run packages/core/src/benchmarks/scan.bench.ts"
```

**Exit criteria:** Benchmarks run without error, produce timing output. Baseline numbers documented in a comment in each bench file.

---

## Implementation Order and Dependencies

```
T1 (core unit tests)
├── T1.1 symlink.test.ts       — no deps, start here
├── T1.2 remove.test.ts        — depends on symlink working
└── T1.3 update.test.ts        — depends on remove/install working

T2 (agent invoke tests)
├── T2.1 createMockAgentBinary — foundation for T2.2–T2.4
├── T2.2 factory.test.ts       — depends on T2.1
├── T2.3 custom.test.ts        — depends on T2.1
└── T2.4 ollama.test.ts        — depends on T2.1

T3 (agent mode E2E)            — depends on T2.1 pattern being established
├── T3.2 install.agent-mode    — no dep on T2 directly, but sequenced after
└── T3.3 update.agent-mode

T4 (contracts)                 — independent, can run in parallel with T1/T2
├── T4.1 agent output snapshots
├── T4.2 installed.json compat
└── T4.3 config round-trip

T5 (edge cases)                — builds on T1 being complete
T6 (benchmarks)                — fully independent, can be done anytime
```

Recommended sequence: **T1 → T2 → T4 → T3 → T5 → T6**

T1 and T4 are the highest-value/lowest-effort phases and should be done first.

---

## Test Count Targets

| Phase | New test files | Estimated new tests |
|-------|---------------|---------------------|
| T1    | 3             | ~60                 |
| T2    | 4 + 1 util    | ~30                 |
| T3    | 2             | ~20                 |
| T4    | 1 + extended  | ~15                 |
| T5    | extended      | ~20                 |
| T6    | 1 bench file  | N/A (benchmarks)    |
| **Total** | **11**    | **~145 new tests**  |

Expected total after all phases: ~580 tests.
