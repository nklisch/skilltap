# Design: Git URL Protocol Fallback

## Overview

When a git clone fails due to authentication, skilltap should automatically retry with the alternate URL protocol (HTTPS → SSH, SSH → HTTPS). This is transparent at the `clone()` layer in `git.ts`, so all callers — skill installs, tap operations, doctor self-heal — get fallback resilience for free.

When fallback succeeds, the effective URL is returned so callers can persist it (installed.json, config.toml tap URL).

## Implementation Units

### Unit 1: `flipUrlProtocol()` — URL conversion utility

**File**: `packages/core/src/git.ts`

```typescript
/**
 * Convert between HTTPS and SSH git URL forms.
 * Returns null if the URL isn't a recognized pattern or has no alternate form.
 *
 * Supported conversions:
 *   https://github.com/owner/repo.git  ↔  git@github.com:owner/repo.git
 *   https://gitlab.com/owner/repo.git  ↔  git@gitlab.com:owner/repo.git
 *   ssh://git@host/owner/repo.git      →  https://host/owner/repo.git
 *   (any host, not just GitHub/GitLab)
 */
export function flipUrlProtocol(url: string): string | null;
```

**Implementation Notes**:
- HTTPS pattern: `https://<host>/<path>.git` or without `.git` suffix
- SSH scp-style pattern: `git@<host>:<path>.git`
- SSH URL pattern: `ssh://git@<host>/<path>.git`
- Strip trailing `.git` for matching, re-add for output consistency
- `http://` URLs are not flipped (rare, usually intentional non-HTTPS)
- Local paths (`/...`, `./...`) and `npm:` sources return null (no flip possible)
- Must handle paths with multiple segments (e.g. `gitlab.com/group/subgroup/repo`)

**Logic**:
```typescript
export function flipUrlProtocol(url: string): string | null {
  // HTTPS → SSH scp-style
  const httpsMatch = url.match(/^https:\/\/([^/]+)\/(.+?)(?:\.git)?$/);
  if (httpsMatch) {
    const [, host, path] = httpsMatch;
    return `git@${host}:${path}.git`;
  }

  // SSH scp-style → HTTPS
  const sshScpMatch = url.match(/^git@([^:]+):(.+?)(?:\.git)?$/);
  if (sshScpMatch) {
    const [, host, path] = sshScpMatch;
    return `https://${host}/${path}.git`;
  }

  // SSH URL → HTTPS
  const sshUrlMatch = url.match(/^ssh:\/\/git@([^/]+)\/(.+?)(?:\.git)?$/);
  if (sshUrlMatch) {
    const [, host, path] = sshUrlMatch;
    return `https://${host}/${path}.git`;
  }

  return null;
}
```

**Acceptance Criteria**:
- [ ] `flipUrlProtocol("https://github.com/owner/repo.git")` → `"git@github.com:owner/repo.git"`
- [ ] `flipUrlProtocol("https://github.com/owner/repo")` → `"git@github.com:owner/repo.git"`
- [ ] `flipUrlProtocol("git@github.com:owner/repo.git")` → `"https://github.com/owner/repo.git"`
- [ ] `flipUrlProtocol("ssh://git@github.com/owner/repo.git")` → `"https://github.com/owner/repo.git"`
- [ ] `flipUrlProtocol("git@gitlab.com:group/subgroup/repo.git")` → `"https://gitlab.com/group/subgroup/repo.git"`
- [ ] `flipUrlProtocol("https://gitlab.com/group/subgroup/repo.git")` → `"git@gitlab.com:group/subgroup/repo.git"`
- [ ] `flipUrlProtocol("/local/path")` → `null`
- [ ] `flipUrlProtocol("npm:@scope/pkg")` → `null`
- [ ] `flipUrlProtocol("http://example.com/repo.git")` → `null`

---

### Unit 2: `isAuthError()` — auth failure detection helper

**File**: `packages/core/src/git.ts`

```typescript
/** Returns true if a GitError indicates an authentication or access failure. */
function isAuthError(error: GitError): boolean;
```

**Implementation Notes**:
- Extract from existing `clone()` error handling — the auth-related stderr checks already exist in the function
- Match against known git stderr patterns:
  - `"Authentication failed"` — HTTPS credential rejection
  - `"Permission denied"` — SSH key rejection
  - `"Could not read from remote repository"` — SSH access denied
  - `"terminal prompts disabled"` — git credential helper can't prompt (common in CI/non-interactive)
- The error message from `clone()` already includes stderr content, so match against `error.message`

```typescript
const AUTH_PATTERNS = [
  "Authentication failed",
  "Permission denied",
  "Could not read from remote repository",
  "terminal prompts disabled",
];

function isAuthError(error: GitError): boolean {
  return AUTH_PATTERNS.some((p) => error.message.includes(p));
}
```

**Acceptance Criteria**:
- [ ] Returns `true` for `new GitError("Authentication failed for 'https://...'")`
- [ ] Returns `true` for `new GitError("Repository not found or SSH access denied: ...")`
- [ ] Returns `false` for `new GitError("Repository not found: ...")`
- [ ] Returns `false` for `new GitError("git clone failed: not a git repository")`

---

### Unit 3: Refactor `clone()` — add fallback retry and return effective URL

**File**: `packages/core/src/git.ts`

```typescript
export type CloneResult = {
  /** The URL that was actually used to clone (may differ from input if fallback succeeded). */
  effectiveUrl: string;
};

export async function clone(
  url: string,
  dest: string,
  opts?: CloneOptions,
): Promise<Result<CloneResult, GitError>>;
```

**Implementation Notes**:
- Extract the current clone body into a private `tryClone()` that returns `Result<void, GitError>` (same as current `clone()`)
- New `clone()` calls `tryClone(url)`. On auth error, computes `flipUrlProtocol(url)`. If an alternate exists, calls `tryClone(alt)` after cleaning `dest` (a failed clone may leave a partial directory)
- Must remove the partial clone directory before retrying — `git clone` refuses to clone into a non-empty directory
- Debug log the fallback attempt
- The existing detailed error messages in clone (Authentication failed, Permission denied, etc.) remain in `tryClone()` — they provide good hints. The outer `clone()` only catches auth errors to retry, and if the retry also fails, returns the *retry's* error (since that's the last thing tried)

```typescript
async function tryClone(
  url: string,
  dest: string,
  opts?: CloneOptions,
): Promise<Result<void, GitError>> {
  // ... existing clone() body (moved here unchanged)
}

export async function clone(
  url: string,
  dest: string,
  opts?: CloneOptions,
): Promise<Result<CloneResult, GitError>> {
  const result = await tryClone(url, dest, opts);
  if (result.ok) return ok({ effectiveUrl: url });

  if (!isAuthError(result.error)) return result;

  const alt = flipUrlProtocol(url);
  if (!alt) return result;

  debug("auth failed, retrying with alternate URL", { original: url, fallback: alt });

  // Clean partial clone before retry
  await rm(dest, { recursive: true, force: true }).catch(() => {});

  const retryResult = await tryClone(alt, dest, opts);
  if (retryResult.ok) return ok({ effectiveUrl: alt });

  // Both failed — return original error (more informative)
  return result;
}
```

**Key decision**: When both URLs fail, return the *original* error, not the retry error. The original URL is what the user configured, so the original error is more relevant.

**Acceptance Criteria**:
- [ ] Successful clone on first try returns `{ effectiveUrl: url }` (same URL passed in)
- [ ] Auth failure with HTTPS URL retries with SSH; if SSH succeeds, returns `{ effectiveUrl: "git@..." }`
- [ ] Auth failure with SSH URL retries with HTTPS; if HTTPS succeeds, returns `{ effectiveUrl: "https://..." }`
- [ ] Auth failure with no alternate (e.g. local path) returns original error
- [ ] Both URLs fail → returns original error
- [ ] Non-auth errors (repo not found) do NOT trigger fallback
- [ ] Partial clone directory is cleaned before retry

---

### Unit 4: Update install.ts to persist the effective URL

**File**: `packages/core/src/install.ts`

**Changes**:
- Where `clone()` is called (~line 430), destructure `effectiveUrl` from the result
- Use `effectiveUrl` instead of `resolved.url` when building the `InstalledSkill` record
- The `repo` field in installed.json will reflect the URL that actually worked

```typescript
// Before (current):
const cloneResult = await clone(resolved.url, tmpDir, { branch: effectiveRef, depth: 1 });
if (!cloneResult.ok) return cloneResult;

// After:
const cloneResult = await clone(resolved.url, tmpDir, { branch: effectiveRef, depth: 1 });
if (!cloneResult.ok) return cloneResult;
const cloneUrl = cloneResult.value.effectiveUrl;

// ... later, when building the installed record:
// Use cloneUrl instead of resolved.url for the repo field
```

**Implementation Notes**:
- `resolved.url` is currently used in several places downstream: the installed record's `repo` field, trust resolution, tap skill matching. Only the `repo` field in installed.json should use `cloneUrl`. Trust resolution and tap matching should continue using the original `resolved.url` (taps store the canonical URL, not the fallback).
- Specifically, the `repo` field in the `InstalledSkill` record (around line 545-570 in install.ts) should use `cloneUrl`.

**Acceptance Criteria**:
- [ ] When clone succeeds on primary URL, installed.json `repo` matches the original URL
- [ ] When clone falls back to SSH, installed.json `repo` records the SSH URL
- [ ] Trust resolution still uses the original resolved URL (not the fallback)
- [ ] Tap skill matching still uses the original resolved URL

---

### Unit 5: Update taps.ts to persist the effective URL

**File**: `packages/core/src/taps.ts`

**Changes**:
- In `addTap()`, after cloning, if `effectiveUrl` differs from the input `url`, update the tap entry in config with the working URL
- In `updateTap()`, the clone-based self-heal path should also use `effectiveUrl`
- In `ensureBuiltinTap()`, the builtin URL is a constant so there's no persistent record to update — just use whatever URL works

```typescript
// addTap() — around line 171:
const cloneResult = await clone(url, dest, { depth: 1 });
if (!cloneResult.ok) return cloneResult;
const effectiveUrl = cloneResult.value.effectiveUrl;

// Use effectiveUrl in the config entry:
config.taps.push({ name, url: effectiveUrl, type: "git" });
```

```typescript
// updateTap() — self-heal clone path around line 309:
const cloneResult = await clone(tap.url, dir, { depth: 1 });
if (!cloneResult.ok) return cloneResult;
// If URL changed, update config:
if (cloneResult.value.effectiveUrl !== tap.url) {
  tap.url = cloneResult.value.effectiveUrl;
  await saveConfig(config);
}
```

**Acceptance Criteria**:
- [ ] `skilltap tap add` with HTTPS URL that needs SSH records SSH URL in config.toml
- [ ] `skilltap tap update` self-heal clone with fallback updates the tap URL in config
- [ ] Built-in tap clone with fallback works (no config mutation needed for builtin)

---

### Unit 6: Update doctor.ts clone call

**File**: `packages/core/src/doctor.ts`

**Changes**:
- The doctor self-heal clone (~line 605) ignores the return value. Just adjust for the new return type.

```typescript
// Before:
await clone(tapUrl, dir, { depth: 1 });

// After (return type changed, but doctor doesn't need the effective URL):
await clone(tapUrl, dir, { depth: 1 });
// No change needed — clone() still returns Result, doctor checks .ok
```

**Acceptance Criteria**:
- [ ] Doctor self-heal still works — just needs to compile with new return type

---

## Implementation Order

1. **Unit 1**: `flipUrlProtocol()` — pure function, no dependencies, fully testable in isolation
2. **Unit 2**: `isAuthError()` — pure function, depends only on GitError type
3. **Unit 3**: Refactor `clone()` — depends on Units 1 & 2. This is the core change.
4. **Unit 4**: Update `install.ts` — depends on Unit 3's new return type
5. **Unit 5**: Update `taps.ts` — depends on Unit 3's new return type
6. **Unit 6**: Update `doctor.ts` — depends on Unit 3's new return type

Units 4, 5, and 6 are independent of each other and can be implemented in parallel after Unit 3.

## Testing

### Unit Tests: `packages/core/src/git.test.ts`

**`flipUrlProtocol()`** — pure function tests:
```typescript
describe("flipUrlProtocol", () => {
  test("HTTPS → SSH scp-style", () => {
    expect(flipUrlProtocol("https://github.com/owner/repo.git")).toBe("git@github.com:owner/repo.git");
  });
  test("HTTPS without .git suffix", () => {
    expect(flipUrlProtocol("https://github.com/owner/repo")).toBe("git@github.com:owner/repo.git");
  });
  test("SSH scp-style → HTTPS", () => {
    expect(flipUrlProtocol("git@github.com:owner/repo.git")).toBe("https://github.com/owner/repo.git");
  });
  test("SSH URL → HTTPS", () => {
    expect(flipUrlProtocol("ssh://git@github.com/owner/repo.git")).toBe("https://github.com/owner/repo.git");
  });
  test("GitLab nested group path", () => {
    expect(flipUrlProtocol("git@gitlab.com:group/sub/repo.git")).toBe("https://gitlab.com/group/sub/repo.git");
  });
  test("non-git URL returns null", () => {
    expect(flipUrlProtocol("/local/path")).toBeNull();
    expect(flipUrlProtocol("npm:@scope/pkg")).toBeNull();
    expect(flipUrlProtocol("http://example.com/repo.git")).toBeNull();
  });
});
```

**`isAuthError()`** — pattern matching tests:
```typescript
describe("isAuthError", () => {
  // Test against GitError instances with known stderr patterns
  test("matches HTTPS auth failure", ...);
  test("matches SSH permission denied", ...);
  test("does not match repo-not-found", ...);
});
```

**`clone()` fallback** — integration tests with real git repos are tricky (can't simulate auth failure easily). Test via:
1. Mock approach: Use injectable deps pattern — extract `tryClone` and inject a mock in tests
2. Or: Test the fallback indirectly via the pure helper functions, and rely on one manual/CI integration test

**Recommended**: Keep integration tests focused on `flipUrlProtocol` (pure) and `isAuthError` (pure). The `clone()` wiring is simple enough that the unit tests on its components plus existing clone integration tests provide sufficient coverage.

### Integration Tests

Existing `git.test.ts` clone tests must be updated for the new return type:
```typescript
// Before:
const result = await clone(repo.path, `${dest}/clone`);
expect(result.ok).toBe(true);

// After:
const result = await clone(repo.path, `${dest}/clone`);
expect(result.ok).toBe(true);
if (!result.ok) return;
expect(result.value.effectiveUrl).toBe(repo.path); // local paths don't flip
```

## Verification Checklist

```bash
# Type check
bun run build

# Run git.ts unit tests
bun test packages/core/src/git.test.ts

# Run install tests
bun test packages/core/src/install.test.ts

# Run tap tests
bun test packages/core/src/taps.test.ts

# Run all tests
bun test
```
