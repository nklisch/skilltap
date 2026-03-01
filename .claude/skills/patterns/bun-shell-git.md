# Pattern: Git via Bun Shell

All git subprocess calls use Bun's `$` template tag with `.quiet()`, and stderr is extracted from errors via a shared `extractStderr()` helper.

## Rationale

Using the system git binary means user authentication (SSH keys, credential helpers, token env vars) works automatically — no library auth configuration needed. `.quiet()` prevents git output from leaking to the CLI's stdout. `extractStderr()` normalizes the various shapes Bun's shell error can take.

## Examples

### Example 1: extractStderr helper
**File**: `packages/core/src/git.ts:17`
```typescript
function extractStderr(e: unknown): string {
  if (e && typeof e === "object" && "stderr" in e) {
    const s = (e as { stderr: unknown }).stderr
    if (s instanceof Uint8Array) return new TextDecoder().decode(s).trim()
    if (typeof s === "string") return s.trim()
  }
  return String(e)
}
```

### Example 2: Standard git operation pattern
**File**: `packages/core/src/git.ts:26`
```typescript
export async function clone(url: string, dest: string): Promise<Result<void, GitError>> {
  try {
    await $`git clone -- ${url} ${dest}`.quiet()
    return ok(undefined)
  } catch (e) {
    return err(new GitError(`git clone failed: ${extractStderr(e)}`))
  }
}
```

### Example 3: Capturing stdout from git
**File**: `packages/core/src/git.ts:52`
```typescript
export async function revParse(dir: string, ref = "HEAD"): Promise<Result<string, GitError>> {
  try {
    const result = await $`git -C ${dir} rev-parse ${ref}`.quiet()
    return ok(result.stdout.toString().trim())
  } catch (e) {
    return err(new GitError(`git rev-parse failed: ${extractStderr(e)}`))
  }
}
```

### Example 4: Test utility git commands (no Result wrapping)
**File**: `packages/test-utils/src/git.ts:3`
```typescript
export async function initRepo(dir: string): Promise<void> {
  await $`git -C ${dir} init`.quiet()
  await $`git -C ${dir} config user.email "test@skilltap.test"`.quiet()
  await $`git -C ${dir} config user.name "Skilltap Test"`.quiet()
}
```

## When to Use

- Any git operation in `packages/core/src/git.ts`
- Test utilities that set up git repos

## When NOT to Use

- Don't use a git library (nodegit, isomorphic-git) — shell is simpler and auth just works
- Don't use `Bun.spawn` — `$` template tag handles quoting and is more readable
- Don't omit `.quiet()` in core — stdout/stderr must not leak

## Common Violations

- Forgetting `.quiet()` in core functions — git progress output appears on CLI stdout
- Not using `extractStderr(e)` — `String(e)` gives unhelpful `[object Object]` messages
- Using `-C` flag inconsistently — always pass directory as `-C ${dir}` rather than `cd`
- Using `Bun.spawn` or `child_process.exec` — use `$` template tag instead
