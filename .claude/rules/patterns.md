# Patterns Index

- **Result type**: `Result<T,E>` discriminated union with `ok()`/`err()` constructors — all fallible core functions return this, never throw → [patterns/result-type.md]
- **Error hierarchy**: `UserError`, `GitError`, `ScanError`, `NetworkError` extend `SkilltapError` with optional `hint` field → [patterns/error-hierarchy.md]
- **Zod boundary**: Zod schema is source of truth for type + validation; `safeParse` + `z.prettifyError` at every data ingestion point; use `.prefault({})` not `.default({})` for nested objects → [patterns/zod-boundary.md]
- **SourceAdapter strategy**: Plain object literals implementing `{ name, canHandle(), resolve() }`; resolver iterates `ADAPTERS[]` in priority order → [patterns/source-adapter.md]
- **Config I/O**: Load = ensureDirs → exists check → read → parse format → safeParse → ok; Save = ensureDirs → serialize → Bun.write → ok → [patterns/config-io.md]
- **Bun shell git**: All git via `` $`git ...`.quiet() `` + `extractStderr(e)` in catch; stdout captured via `result.stdout.toString().trim()` → [patterns/bun-shell-git.md]
- **Fixture repo factory**: `createX()` returns `{ path, cleanup }` — copy static fixture dir, `initRepo`, `commitAll`; always `dot:true` in Bun.Glob.scan → [patterns/test-fixtures.md]
- **Result test assertions**: `expect(result.ok).toBe(true)` then `if (!result.ok) return` guard; `VALID_*` constants with spread for schema variants → [patterns/test-result-assertions.md]
