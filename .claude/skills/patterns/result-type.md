# Pattern: Result Type — Railway-Oriented Error Handling

All fallible core functions return `Result<T, E>` instead of throwing exceptions, enabling explicit error propagation without try/catch at call sites.

## Rationale

Core functions never write to stdout/stderr — the CLI layer owns all output. Returning `Result` keeps errors as first-class values that callers must handle explicitly, and prevents accidental exception leakage across layer boundaries. The discriminated union enables TypeScript exhaustiveness checking.

## Examples

### Example 1: Type definition and constructors
**File**: `packages/core/src/types.ts:1`
```typescript
export type Result<T, E = SkilltapError> = { ok: true; value: T } | { ok: false; error: E }

export function ok<T>(value: T): Result<T, never> {
  return { ok: true, value }
}

export function err<E>(error: E): Result<never, E> {
  return { ok: false, error }
}
```

### Example 2: Function returning Result
**File**: `packages/core/src/git.ts:26`
```typescript
export async function clone(url: string, dest: string, opts: CloneOptions = {}): Promise<Result<void, GitError>> {
  try {
    await $`git clone ${flags} -- ${url} ${dest}`.quiet()
    return ok(undefined)
  } catch (e) {
    return err(new GitError(`git clone failed: ${extractStderr(e)}`))
  }
}
```

### Example 3: Early return on error (chaining)
**File**: `packages/core/src/config.ts:92`
```typescript
const dirsResult = await ensureDirs()
if (!dirsResult.ok) return dirsResult

const exists = await Bun.file(configPath).exists()
if (!exists) {
  await Bun.write(configPath, CONFIG_TEMPLATE)
  return ok(structuredClone(DEFAULT_CONFIG))
}
```

### Example 4: Adapter returning Result
**File**: `packages/core/src/adapters/github.ts:30`
```typescript
async resolve(source: string): Promise<Result<ResolvedSource, UserError>> {
  const match = source.match(GITHUB_PATTERN)
  if (!match) return err(new UserError("Invalid GitHub source"))
  return ok({ url, adapter: "github" })
}
```

## When to Use

- Any function in `packages/core/` that can fail (I/O, validation, git, network)
- Adapter `resolve()` methods
- Config load/save functions

## When NOT to Use

- Test utility functions in `@skilltap/test-utils` — those throw directly (simpler for tests)
- Pure transformations that cannot fail
- CLI layer unwraps Results and handles them with user-facing output

## Common Violations

- Throwing errors from core functions — breaks the layer boundary contract
- Using `Promise<T | null>` — loses error context
- Nesting try/catch inside try/catch — chain with early returns instead
