# Pattern: Git via Bun Shell

All git subprocess calls use Bun's `$` template tag with `.quiet()`, wrapped in a generic `wrapGit<T>()` helper that normalizes errors into `Result<T, GitError>`.

## Rationale

Using the system git binary means user authentication (SSH keys, credential helpers, token env vars) works automatically — no library auth configuration needed. `.quiet()` prevents git output from leaking to the CLI's stdout. `wrapGit<T>()` eliminates repetitive try/catch blocks by wrapping any async git operation into a `Result<>` with extracted stderr.

## Examples

### Example 1: extractStderr helper
**File**: `packages/core/src/git.ts:16`
```typescript
function extractStderr(e: unknown): string {
  if (e instanceof Error && "stderr" in e) {
    const raw = (e as { stderr: unknown }).stderr;
    if (raw instanceof Uint8Array) return new TextDecoder().decode(raw).trim();
    return String(raw).trim();
  }
  return String(e);
}
```

### Example 2: wrapGit<T>() generic wrapper
**File**: `packages/core/src/git.ts:25`
```typescript
async function wrapGit<T>(
  fn: () => Promise<T>,
  msg: string,
  hint?: string,
): Promise<Result<T, GitError>> {
  try {
    return ok(await fn());
  } catch (e) {
    return err(
      new GitError(`${msg}: ${extractStderr(e)}`, hint ? { hint } : undefined),
    );
  }
}
```

### Example 3: Simple void operation (clone)
**File**: `packages/core/src/git.ts:39`
```typescript
export async function clone(url: string, dest: string, opts?: CloneOptions): Promise<Result<void, GitError>> {
  const flags: string[] = ["--depth", String(opts?.depth ?? 1)];
  if (opts?.branch) flags.push("--branch", opts.branch);
  return wrapGit(
    () => $`git clone ${flags} -- ${url} ${dest}`.quiet().then(() => undefined),
    "git clone failed",
    "Check that the URL is correct and you have access.",
  );
}
```

### Example 4: Capturing stdout (revParse)
**File**: `packages/core/src/git.ts:157`
```typescript
export async function revParse(dir: string, ref = "HEAD"): Promise<Result<string, GitError>> {
  return wrapGit(
    () => $`git -C ${dir} rev-parse ${ref}`.quiet().then((r) => r.stdout.toString().trim()),
    "git rev-parse failed",
  );
}
```

### Example 5: Complex multi-query operation (diffStat)
**File**: `packages/core/src/git.ts:97`
```typescript
export async function diffStat(dir, from, to, pathSpec?): Promise<Result<DiffStat, GitError>> {
  const extra = pathSpec ? ["--", pathSpec] : [];
  return wrapGit(async () => {
    const numstatOut = await $`git -C ${dir} diff --numstat ${from}..${to} ${extra}`.quiet()
      .then((r) => r.stdout.toString().trim());
    const nameStatusOut = await $`git -C ${dir} diff --name-status ${from}..${to} ${extra}`.quiet()
      .then((r) => r.stdout.toString().trim());
    // ...parse and combine into DiffStat
    return { filesChanged: files.length, insertions, deletions, files };
  }, "git diff stat failed");
}
```

### Example 6: Test utility git commands (no Result wrapping)
**File**: `packages/test-utils/src/git.ts:3`
```typescript
export async function initRepo(dir: string): Promise<void> {
  await $`git -C ${dir} init`.quiet()
  await $`git -C ${dir} config user.email "test@skilltap.test"`.quiet()
  await $`git -C ${dir} config user.name "Skilltap Test"`.quiet()
}
```

## When to Use

- Any git operation in `packages/core/src/git.ts` — always use `wrapGit<T>()`
- Test utilities that set up git repos — use `$` directly (no `wrapGit`, these are test helpers that throw on failure)

## When NOT to Use

- Don't use a git library (nodegit, isomorphic-git) — shell is simpler and auth just works
- Don't use `Bun.spawn` for git — `$` template tag handles quoting and is more readable
- Don't omit `.quiet()` in core — git progress output must not leak

## Common Violations

- Inlining try/catch instead of using `wrapGit<T>()` — leads to inconsistent error messages
- Forgetting `.quiet()` in core functions — git progress output appears on CLI stdout
- Not using `extractStderr(e)` — `String(e)` gives unhelpful `[object Object]` messages
- Using `-C` flag inconsistently — always pass directory as `-C ${dir}` rather than `cd`
