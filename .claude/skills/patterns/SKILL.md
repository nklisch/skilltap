---
name: patterns
description: "Project code patterns and conventions. Auto-loads when implementing, designing, verifying, or reviewing code. Provides detailed pattern definitions with code examples. Consult this whenever writing new code, reviewing changes, or trying to understand how this project structures things."
user-invocable: false
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
- **zod-boundary.md** — Zod schema as single source of truth for types + validation; `safeParse` + `z.prettifyError` at every data boundary; `.prefault({})` for nested defaults
- **config-io.md** — Config/state load-save algorithm: ensureDirs → exists check → read → parse → Zod validate → Result

### Adapter Patterns
- **source-adapter.md** — `SourceAdapter` strategy pattern: plain object literals with `canHandle()` + `resolve()`, iterated by a priority-ordered resolver
- **agent-adapter-strategy.md** — `AgentAdapter` interface with `detect()`/`invoke()`, factory functions for CLI/custom/Ollama adapters, three-priority resolution via `resolveAgent()`

### Command Patterns
- **callback-driven-options.md** — Core functions accept typed option objects with async callbacks for decision points; omitting callback = auto-proceed; 10+ callback fields across `InstallOptions`/`UpdateOptions`
- **policy-composition.md** — `composePolicy(config, flags)` pure function centralizes all config + CLI flag precedence into `EffectivePolicy`; used for early command branching
- **agent-mode-branching.md** — CLI commands fork into `runAgentMode()` (plain text, auto-accept, hard-fail) vs `runInteractiveMode()` (spinners, prompts, ANSI) based on policy

### Git & Security
- **bun-shell-git.md** — All git operations via `wrapGit<T>()` wrapper + Bun's `$` template tag with `.quiet()`; `extractStderr()` for consistent error extraction
- **security-detector-composition.md** — 7 independent detector functions composed in a for-loop inside `scanStatic()`; `StaticWarning` extends `PatternMatch` with a `file` field
- **install-result-with-warnings.md** — `installSkill()` returns `InstallResult { records, warnings, semanticWarnings }`; optional callbacks for per-skill interception; `skipScan: true` in tests

### Testing
- **test-fixtures.md** — Fixture repo factories: `createX()` returns `{ path, cleanup }`; copies static fixtures, initializes git repo, commits; always `dot:true` in Bun.Glob.scan
- **test-result-assertions.md** — Result assertion pattern: `expect(result.ok).toBe(true)` + discriminated union guard; `VALID_*` constants with spread for schema test variants
- **cli-subprocess-testing.md** — CLI integration tests use `Bun.spawn` with `SKILLTAP_HOME`/`XDG_CONFIG_HOME` env vars for isolation; `stdin: "pipe"` for non-TTY detection tests
