# Pattern: Result Assertions in Tests

Tests check `result.ok` with `expect().toBe(true/false)` then use a discriminated union guard to safely access `result.value` or `result.error` — no type assertions needed.

## Rationale

`Result<T, E>` is a discriminated union. TypeScript narrows the type after `if (result.ok)` — accessing `.value` or `.error` without the guard would be a type error. Using `expect().toBe(true)` reports the assertion failure clearly before the guard.

## Examples

### Example 1: Checking success and accessing value
**File**: `packages/core/src/git.test.ts:21`
```typescript
const result = await clone(repo.path, dest + "/clone")
expect(result.ok).toBe(true)
if (!result.ok) return   // guard narrows type; early return if assertion already failed

expect(await Bun.file(dest + "/clone/SKILL.md").exists()).toBe(true)
```

### Example 2: Checking failure and accessing error
**File**: `packages/core/src/adapters/local.test.ts:38`
```typescript
const result = await localAdapter.resolve("/nonexistent/path")
expect(result.ok).toBe(false)
if (result.ok) return   // guard; early return

expect(result.error.message).toContain("does not exist")
```

### Example 3: Inline VALID_* constants with spread for variations
**File**: `packages/core/src/schemas/installed.test.ts:4`
```typescript
const VALID_SKILL = {
  name: "commit-helper",
  repo: "https://gitea.example.com/nathan/commit-helper",
  ref: "v1.2.0",
  sha: "abc123def456",
  scope: "global" as const,
  installedAt: "2026-02-28T12:00:00.000Z",
}

test("accepts linked skill with repo null", () => {
  const result = InstalledSkillSchema.safeParse({ ...VALID_SKILL, repo: null, scope: "linked" as const })
  expect(result.success).toBe(true)
})

test("rejects missing name", () => {
  const { name: _, ...noName } = VALID_SKILL
  const result = InstalledSkillSchema.safeParse(noName)
  expect(result.success).toBe(false)
})
```

### Example 4: beforeEach env setup with afterEach restore
**File**: `packages/core/src/config.test.ts:10`
```typescript
let tmpDir: string
let savedXdg: string | undefined

beforeEach(async () => {
  tmpDir = await mkdtemp(join(tmpdir(), "skilltap-test-"))
  savedXdg = process.env.XDG_CONFIG_HOME
  process.env.XDG_CONFIG_HOME = tmpDir
})

afterEach(async () => {
  if (savedXdg !== undefined) {
    process.env.XDG_CONFIG_HOME = savedXdg
  } else {
    delete process.env.XDG_CONFIG_HOME
  }
  await rm(tmpDir, { recursive: true, force: true })
})
```

## When to Use

- Any test that calls a core function returning `Result<T, E>`
- Schema tests use `safeParse().success` (same discriminated union shape)
- `VALID_*` constants with spread whenever testing multiple variants of a schema

## When NOT to Use

- Don't use `(result as { ok: true }).value` — use the guard pattern instead
- Don't skip the `expect(result.ok).toBe(true)` before the guard — the guard alone gives a cryptic type error on failure

## Common Violations

- Accessing `result.value` without `expect(result.ok).toBe(true)` first — test passes silently with wrong data
- Using `as const` only on the whole object instead of on discriminant fields like `scope`
- Forgetting to restore env vars in `afterEach` — bleeds into subsequent tests
