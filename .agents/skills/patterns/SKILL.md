---
name: patterns
description: "Project code patterns and conventions. Auto-loads when implementing,
  designing, verifying, or reviewing code. Provides detailed pattern definitions
  with code examples. Consult this whenever writing new code, reviewing changes,
  or trying to understand how this project structures things."
user-invocable: false
allowed-tools: Read, Glob, Grep
---

# Project Patterns Reference

This skill contains documented code patterns for this project — recurring structures, shared abstractions, and architectural approaches that keep the codebase consistent.

Each pattern file has a rationale explaining *why* the pattern exists, concrete code examples with file references, and guidance on when to use it (and when not to).

## How to Use

When writing new code or reviewing changes, check if an established pattern applies. If it does, follow it. If you have a good reason to deviate, note why.

The dense index at `.claude/rules/patterns.md` loads automatically and provides one-line summaries with pointers to full pattern files. Read the individual pattern file when you need full details.

## Available Patterns

### Core Architecture
- **result-type.md** — `Result<T,E>` discriminated union with `ok()`/`err()` constructors for railway-oriented error handling across all core functions
- **error-hierarchy.md** — `SkilltapError` base class with typed subclasses (`UserError`, `GitError`, `ScanError`, `NetworkError`) and optional `hint` field
- **zod-boundary.md** — Zod schema as single source of truth for types + validation; `safeParse` + `z.prettifyError` at every data boundary; `.prefault({})` for nested defaults; `parseWithResult()` helper
- **config-io.md** — Config/state load-save algorithm: ensureDirs → exists check → read → parse → Zod validate → Result; state.json uses `loadState`/`saveState`
- **json-state-io.md** — `loadJsonState<T>` and `saveJsonState` generic helpers; all state modules delegate to these, no ad-hoc JSON I/O

### Adapter Patterns
- **source-adapter.md** — `SourceAdapter` strategy pattern: plain object literals with `canHandle()` + `resolve()`, iterated by a priority-ordered resolver
- **agent-adapter-strategy.md** — `AgentAdapter` interface with `detect()`/`invoke()`, factory functions for CLI/custom/Ollama adapters, three-priority resolution via `resolveAgent()`
- **adapter-driven-branching.md** — `resolved.adapter` from `resolveSource()` gates source-type-specific logic (npm vs git vs local) throughout install, update, and trust flows

### Command Patterns
- **output-interface.md** — `setupOutput(args)` → `Output` handle; 3 modes (tty/plain/json); all command output goes through `out.*` methods; replaces old agent-mode-branching split
- **callback-driven-options.md** — Core functions accept typed option objects with async callbacks for decision points; omitting callback = auto-proceed; `out?: Output` for progress
- **policy-composition.md** — `composePolicy(config, flags)` → `EffectivePolicy`; `composePolicyForSource` adds trust overlay; `loadPolicyOrExit()` is CLI-layer entry point
- **scope-base.md** — `scopeBase(scope, projectRoot?)` pure helper — single-source scope-to-base-dir; use derived helpers (`skillInstallDir`, etc.) for full paths

### State Management
- **apply-state-change.md** — `applySkillStateChange({scope, projectRoot, mutate, manifestSync?})` atomic load→mutate→save for skills[]; fires manifest sync hooks on diff

### Git & Security
- **bun-shell-git.md** — All git operations via `wrapGit<T>()` wrapper + Bun's `$` template tag with `.quiet()`; `extractStderr()` for consistent error extraction
- **security-detector-composition.md** — 7 independent detector functions composed in a for-loop inside `scanStatic()`; `StaticWarning` extends `PatternMatch` with a `file` field
- **install-result-with-warnings.md** — `installSkill()` returns `InstallResult { records, warnings, semanticWarnings, updates, pluginRecord? }`; unified `onWarnings(warnings, kind, name)` callback

### Testing
- **test-fixtures.md** — Fixture repo factories: `createX()` returns `{ path, cleanup }`; copies static fixtures, initializes git repo, commits; always `dot:true` in Bun.Glob.scan
- **test-result-assertions.md** — Result assertion pattern: `expect(result.ok).toBe(true)` + discriminated union guard; `VALID_*` constants with spread for schema test variants
- **test-env-isolation.md** — `createTestEnv()` from `@skilltap/test-utils` returns `{ homeDir, configDir, cleanup() }`; replaces per-test manual env save/restore
- **cli-subprocess-testing.md** — Use `runSkilltap(args, homeDir, configDir)` + `cliCmd()` from `@skilltap/test-utils`; routes to compiled binary when `SKILLTAP_TEST_BIN` set

### Trust & Source Handling
- **injectable-dependencies.md** — Core functions with external I/O accept `_dep = realImpl` optional params; tests inject mocks as 2nd/3rd args; private `type Fn = typeof realFn` aliases enforce signature compatibility
- **graceful-fallback-chain.md** — Optional verifiers return `T | null` not `Result`; caller cascades through priority tiers (provenance → publisher → curated → unverified); outer `try/catch` guarantees a valid result always
- **single-source-definitions.md** — One authoritative constant per enumerable concept; agent metadata in `symlink.ts`; config enum arrays in `schemas/config.ts`

### Deprecated / Removed
- **agent-mode-branching.md** — ⚠️ REMOVED. `policy.agentMode` and `runAgentMode()`/`runInteractiveMode()` no longer exist. See output-interface.md.
