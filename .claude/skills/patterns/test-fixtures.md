# Pattern: Fixture Repo Factory

Integration tests use factory functions that create real git repositories in temp directories from static fixture templates, returning a resource object with a `cleanup()` callback.

## Rationale

Integration tests need real git repos on disk — mocking git is fragile and complex. Static fixture directories in `packages/test-utils/fixtures/` provide repeatable known-good repo contents. The `FixtureRepo` return type with `cleanup()` keeps resource management explicit.

## Examples

### Example 1: FixtureRepo type and factory function
**File**: `packages/test-utils/src/fixtures.ts:7`
```typescript
type FixtureRepo = {
  path: string
  cleanup: () => Promise<void>
}

export async function createStandaloneSkillRepo(): Promise<FixtureRepo> {
  const path = await makeTmpDir()
  await copyFixtureDir("standalone-skill", path)
  await initRepo(path)
  await commitAll(path)
  return { path, cleanup: () => removeTmpDir(path) }
}
```

### Example 2: All factories follow identical structure
**File**: `packages/test-utils/src/fixtures.ts:30`
```typescript
export async function createMultiSkillRepo(): Promise<FixtureRepo> {
  const path = await makeTmpDir()
  await copyFixtureDir("multi-skill-repo", path)
  await initRepo(path)
  await commitAll(path)
  return { path, cleanup: () => removeTmpDir(path) }
}

export async function createSampleTap(): Promise<FixtureRepo> {
  const path = await makeTmpDir()
  await copyFixtureDir("sample-tap", path)
  await initRepo(path)
  await commitAll(path)
  return { path, cleanup: () => removeTmpDir(path) }
}
```

### Example 3: Fixture file copying with Bun.Glob + dot:true
**File**: `packages/test-utils/src/fixtures.ts:12`
```typescript
async function copyFixtureDir(fixtureName: string, destDir: string): Promise<void> {
  const srcDir = join(FIXTURES_DIR, fixtureName)
  const glob = new Bun.Glob("**/*")
  for await (const relPath of glob.scan({ cwd: srcDir, onlyFiles: true, dot: true })) {
    const src = join(srcDir, relPath)
    const dest = join(destDir, relPath)
    await Bun.write(dest, Bun.file(src))
  }
}
```

### Example 4: Usage in integration test with try/finally
**File**: `packages/core/src/scanner.test.ts:100`
```typescript
test("returns exactly 1 valid skill from standalone repo", async () => {
  const repo = await createStandaloneSkillRepo()
  try {
    const skills = await scan(repo.path)
    expect(skills).toHaveLength(1)
    expect(skills[0]!.name).toBe("standalone-skill")
    expect(skills[0]!.valid).toBe(true)
  } finally {
    await repo.cleanup()
  }
})
```

### Example 5: Nullable var + afterEach for shared repo across tests
**File**: `packages/core/src/git.test.ts:5`
```typescript
let repo: { path: string; cleanup: () => Promise<void> } | null = null

afterEach(async () => {
  if (repo) { await repo.cleanup(); repo = null }
})

test("clones a local repo successfully", async () => {
  repo = await createStandaloneSkillRepo()
  // test uses repo.path ...
})
```

## When to Use

- Integration tests that need real git repos
- Any test touching the filesystem at a meaningful level
- When you need multiple fixture templates for different scenarios

## When NOT to Use

- Unit tests of pure functions — no fixtures needed
- Schema validation tests — use inline `VALID_*` constant objects instead

## Common Violations

- Forgetting `cleanup()` — temp dirs accumulate under `/tmp`
- Not using `try/finally` — if test throws, cleanup is skipped
- Using `dot: false` (or omitting `dot`) in `Bun.Glob.scan()` — skips `.agents/`, `.claude/`, and other dotfile directories
- Creating git repos directly in tests instead of using factories — duplicates setup logic
