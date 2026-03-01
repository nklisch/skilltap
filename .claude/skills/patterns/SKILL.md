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

- **result-type.md** — `Result<T,E>` discriminated union with `ok()`/`err()` constructors for railway-oriented error handling across all core functions
- **error-hierarchy.md** — `SkilltapError` base class with typed subclasses (`UserError`, `GitError`, `ScanError`, `NetworkError`) and optional `hint` field
- **zod-boundary.md** — Zod schema as single source of truth for types + validation; `safeParse` + `z.prettifyError` at every data boundary; `.prefault({})` for nested defaults
- **source-adapter.md** — `SourceAdapter` strategy pattern: plain object literals with `canHandle()` + `resolve()`, iterated by a priority-ordered resolver
- **config-io.md** — Config/state load-save algorithm: ensureDirs → exists check → read → parse → Zod validate → Result
- **bun-shell-git.md** — All git operations via Bun's `` $`git ...`.quiet() `` with `extractStderr()` helper for consistent error extraction
- **test-fixtures.md** — Fixture repo factories: `createX()` returns `{ path, cleanup }`; copies static fixtures, initializes git repo, commits; always `dot:true` in Bun.Glob.scan
- **test-result-assertions.md** — Result assertion pattern: `expect(result.ok).toBe(true)` + discriminated union guard; `VALID_*` constants with spread for schema test variants
- **security-detector-composition.md** — 7 independent `(content: string) => PatternMatch[]` detector functions composed in a for-loop inside `scanStatic()`; `StaticWarning` extends `PatternMatch` with a `file` field
- **install-result-with-warnings.md** — `installSkill()` returns `InstallResult { records, warnings }`; optional `onWarnings` callback for per-skill interception; `skipScan: true` in tests
