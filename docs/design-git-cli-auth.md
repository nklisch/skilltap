# Design: Use Git CLI Auth for All Remote Interactions

## Overview

Replace all direct `fetch()` calls to git hosts (GitHub API, GitHub Releases) with git CLI-based alternatives that inherit the user's local git authentication. Add a `default_git_host` config key so `owner/repo` shorthand works with Gitea, Forgejo, GitLab, and other self-hosted forges.

### Problem

Three areas bypass the git CLI's authentication:

1. **Self-update version check** — `fetch("https://api.github.com/repos/.../releases/latest")` with no auth headers. Fails behind corporate proxies, hits unauthenticated rate limits (60 req/hr), and is GitHub-only.
2. **Self-update binary download** — `fetch("https://github.com/.../releases/download/...")` with no auth. Fails for private repos or restrictive networks.
3. **GitHub shorthand** — `owner/repo` always resolves to `https://github.com/...`. Users on Gitea/Forgejo/GitLab must type full URLs.

### What stays the same

- **`git.ts`** — Already uses `Bun.$` to shell out to `git`. Correct.
- **`verify-github.ts`** — Uses `gh` CLI. Correct.
- **`npm-registry.ts`** — npm has its own auth model (`.npmrc`). Not a git concern.
- **`registry/client.ts`** — HTTP registries with Bearer auth. Separate protocol.

---

## Implementation Units

### Unit 1: Add `git ls-remote` to `git.ts`

**File**: `packages/core/src/git.ts`

```typescript
/**
 * List remote tags matching a pattern. Returns tag names (without refs/tags/ prefix).
 * Uses git CLI auth — inherits credential helpers, SSH keys, proxy config.
 */
export async function lsRemoteTags(
  url: string,
  pattern?: string,
): Promise<Result<string[], GitError>> {
  const args = pattern ? [pattern] : [];
  return wrapGit(async () => {
    const result = await $`git ls-remote --tags --refs ${url} ${args}`.quiet();
    const output = result.stdout.toString().trim();
    if (!output) return [];
    return output
      .split("\n")
      .map((line) => {
        // Format: "<sha>\trefs/tags/<tagname>"
        const ref = line.split("\t")[1] ?? "";
        return ref.replace("refs/tags/", "");
      })
      .filter(Boolean);
  }, "git ls-remote failed");
}
```

**Implementation Notes**:
- `--refs` excludes dereferenced tag objects (lines ending in `^{}`), keeping output clean.
- Pattern `"v*"` limits network traffic to version tags only.
- Returns `Result<string[], GitError>` per project conventions.
- Inherits all git auth config: SSH keys, credential helpers, `url.insteadOf`, proxy settings.

**Acceptance Criteria**:
- [ ] `lsRemoteTags(url)` returns all tags from a remote repo
- [ ] `lsRemoteTags(url, "v*")` returns only tags matching the glob
- [ ] Returns `GitError` when the URL is unreachable or auth fails
- [ ] Uses `.quiet()` — no git output leaks to stdout

---

### Unit 2: Replace `fetchLatestVersion()` with `git ls-remote`

**File**: `packages/core/src/self-update.ts`

```typescript
import { lsRemoteTags } from "./git";

// Remove: const GITHUB_OWNER = "nklisch";
// Remove: const GITHUB_REPO = "skilltap";
// Add:
const RELEASE_REPO_URL = "https://github.com/nklisch/skilltap.git";

type LsRemoteTagsFn = typeof lsRemoteTags;

/**
 * Fetch the latest version by listing remote git tags.
 * Uses git CLI — inherits user's auth, proxies, and url.insteadOf rewrites.
 */
export async function fetchLatestVersion(
  _lsRemoteTags: LsRemoteTagsFn = lsRemoteTags,
): Promise<string | null> {
  const result = await _lsRemoteTags(RELEASE_REPO_URL, "v*");
  if (!result.ok) return null;

  const tags = result.value;
  if (tags.length === 0) return null;

  // Parse all valid semver tags, find the highest
  let best: [number, number, number] | null = null;
  let bestTag = "";

  for (const tag of tags) {
    const parsed = parseVersion(tag);
    if (!parsed) continue;
    if (
      !best ||
      parsed[0] > best[0] ||
      (parsed[0] === best[0] && parsed[1] > best[1]) ||
      (parsed[0] === best[0] && parsed[1] === best[1] && parsed[2] > best[2])
    ) {
      best = parsed;
      bestTag = tag;
    }
  }

  if (!bestTag) return null;
  return bestTag.startsWith("v") ? bestTag.slice(1) : bestTag;
}
```

**Implementation Notes**:
- The injectable `_lsRemoteTags` param follows the `injectable-dependencies` pattern. Existing tests mock `_fetchLatest` — same mechanism, new name.
- `parseVersion()` already exists in the file — reused to find the highest semver tag.
- `RELEASE_REPO_URL` replaces `GITHUB_OWNER`/`GITHUB_REPO` — single constant for the remote.
- `checkForUpdate()` signature stays the same (accepts `_fetchLatest: FetchLatestFn`). No downstream changes.

**Acceptance Criteria**:
- [ ] `fetchLatestVersion()` returns the highest semver version from remote tags
- [ ] Returns `null` when git is unavailable or network fails (graceful fallback)
- [ ] No `fetch()` calls to `api.github.com` remain
- [ ] Existing `checkForUpdate` tests pass with mock `_fetchLatest`
- [ ] New unit test: mock `_lsRemoteTags` returning `["v1.0.0", "v1.2.0", "v0.9.0"]` → returns `"1.2.0"`

---

### Unit 3: Replace `downloadAndInstall()` binary download with `gh` + `fetch` fallback

**File**: `packages/core/src/self-update.ts`

```typescript
type GhDownloadFn = (
  version: string,
  asset: string,
  destPath: string,
) => Promise<Result<void, UserError>>;

/** Try `gh release download` — returns ok if gh is available and download succeeds. */
async function ghDownload(
  version: string,
  asset: string,
  destPath: string,
): Promise<Result<void, UserError>> {
  try {
    const whichResult = await $`which gh`.quiet();
    const ghPath = whichResult.stdout.toString().trim();
    if (!ghPath) return err(new UserError("gh not found"));

    await $`${ghPath} release download v${version} --repo nklisch/skilltap --pattern ${asset} --dir ${dirname(destPath)} --clobber`.quiet();

    // gh downloads to dir with original filename — rename to destPath
    const downloadedPath = join(dirname(destPath), asset);
    if (downloadedPath !== destPath) {
      await $`mv -f ${downloadedPath} ${destPath}`.quiet();
    }
    return ok(undefined);
  } catch (e) {
    return err(new UserError(`gh download failed: ${extractStderr(e)}`));
  }
}

type FetchFn = (
  url: string | URL,
  init?: { signal?: AbortSignal },
) => Promise<Response>;

/**
 * Download the specified release and atomically replace the running binary.
 * Tries `gh release download` first (inherits gh auth — works for private repos),
 * falls back to direct HTTP fetch (works for public repos without gh).
 */
export async function downloadAndInstall(
  version: string,
  _fetch: FetchFn = fetch,
  _execPath: string = process.execPath,
  _ghDownload: GhDownloadFn = ghDownload,
): Promise<Result<void, UserError>> {
  const asset = getPlatformAsset();
  if (!asset) {
    return err(
      new UserError(
        "Auto-update is not supported on this platform.",
        "Install manually: npm install -g skilltap",
      ),
    );
  }

  const tmpPath = `${_execPath}.update`;

  // Strategy 1: Try gh CLI (inherits auth, works for private repos)
  const ghResult = await _ghDownload(version, asset, tmpPath);

  if (!ghResult.ok) {
    // Strategy 2: Fall back to direct HTTP fetch (public repos)
    const url = `https://github.com/nklisch/skilltap/releases/download/v${version}/${asset}`;
    let response: Response;
    try {
      response = await _fetch(url, { signal: AbortSignal.timeout(60_000) });
    } catch (e) {
      return err(
        new NetworkError(`Download failed: ${e}`) as unknown as UserError,
      );
    }
    if (!response.ok) {
      return err(
        new UserError(
          `Failed to download v${version}: HTTP ${response.status}`,
        ),
      );
    }
    const buffer = await response.arrayBuffer();
    try {
      await Bun.write(tmpPath, buffer);
    } catch (e) {
      Bun.$`rm -f ${tmpPath}`.quiet();
      return err(
        new UserError(
          `Failed to write update: ${extractStderr(e)}`,
          "Try running with sudo, or install via npm: npm install -g skilltap",
        ),
      );
    }
  }

  // Finalize: chmod + atomic move
  try {
    await Bun.$`chmod +x ${tmpPath}`.quiet();
    await Bun.$`mv -f ${tmpPath} ${_execPath}`.quiet();
  } catch (e) {
    Bun.$`rm -f ${tmpPath}`.quiet();
    return err(
      new UserError(
        `Failed to replace binary: ${extractStderr(e)}`,
        "Try running with sudo, or install via npm: npm install -g skilltap",
      ),
    );
  }

  await writeCache(getConfigDir(), version);
  return ok(undefined);
}
```

**Implementation Notes**:
- `_ghDownload` is injectable following the `injectable-dependencies` pattern. Tests can mock it to simulate `gh` not installed / `gh` succeeding.
- `gh release download` uses `--pattern` (exact match) and `--clobber` (overwrite if exists).
- Fallback `fetch()` is identical to current behavior — preserves backward compatibility for users without `gh`.
- `dirname()` import needed from `node:path`.

**Acceptance Criteria**:
- [ ] When `gh` is available and succeeds, binary is downloaded via `gh release download`
- [ ] When `gh` is not available, falls back to `fetch()` (current behavior)
- [ ] When both fail, returns descriptive error
- [ ] Existing `downloadAndInstall` tests pass (they inject `_fetch` and skip `gh`)
- [ ] New test: mock `_ghDownload` returning `ok()` → no `_fetch` call made
- [ ] New test: mock `_ghDownload` returning `err()` → falls back to `_fetch`

---

### Unit 4: Add `default_git_host` to config schema

**File**: `packages/core/src/schemas/config.ts`

```typescript
// Add to ConfigSchema:
export const ConfigSchema = z.object({
  // ... existing fields ...
  /** Default git host for owner/repo shorthand. Defaults to "https://github.com". */
  default_git_host: z.string().default("https://github.com"),
  // ... rest of existing fields ...
});
```

**File**: `packages/core/src/config-keys.ts`

```typescript
// Add to SETTABLE_KEYS:
export const SETTABLE_KEYS: Record<string, SettableKeyDef> = {
  // ... existing entries ...
  default_git_host: { type: "string" },
};
```

**Implementation Notes**:
- Top-level config key (not nested under a section) since it's a global default.
- Default `"https://github.com"` preserves current behavior.
- Users set via: `skilltap config set default_git_host https://gitea.example.com`
- No trailing slash normalization needed — consumers strip it.

**Acceptance Criteria**:
- [ ] Config loads with `default_git_host` defaulting to `"https://github.com"`
- [ ] `config set default_git_host <url>` persists to config.toml
- [ ] `config get default_git_host` returns the stored value
- [ ] Existing config files without the key parse successfully (default applied)

---

### Unit 5: Update GitHub adapter to use `default_git_host`

**File**: `packages/core/src/adapters/github.ts`

```typescript
import { loadConfig } from "../config";
import { err, ok, UserError } from "../types";
import type { SourceAdapter } from "./types";

const LOCAL_PREFIXES = ["./", "/", "~/"];
const URL_PROTOCOLS = ["https://", "http://", "git@", "ssh://"];

/**
 * Create the GitHub shorthand adapter with the configured git host.
 * If config can't be loaded, falls back to https://github.com.
 */
export function createGithubAdapter(gitHost: string): SourceAdapter {
  const host = gitHost.replace(/\/$/, "");
  return {
    name: "github",

    canHandle(source: string): boolean {
      if (source.startsWith("github:")) return true;
      if (LOCAL_PREFIXES.some((p) => source.startsWith(p))) return false;
      if (URL_PROTOCOLS.some((p) => source.startsWith(p))) return false;
      return source.includes("/");
    },

    async resolve(source: string) {
      let s = source.startsWith("github:")
        ? source.slice("github:".length)
        : source;

      let ref: string | undefined;
      const atIdx = s.lastIndexOf("@");
      if (atIdx !== -1) {
        ref = s.slice(atIdx + 1);
        s = s.slice(0, atIdx);
      }

      const parts = s.split("/").filter(Boolean);
      if (parts.length !== 2) {
        return err(
          new UserError(
            `Invalid GitHub source: "${source}"`,
            "Use format: owner/repo or github:owner/repo",
          ),
        );
      }

      const [owner, repo] = parts;
      const url = `${host}/${owner}/${repo}.git`;
      return ok({ url, ...(ref ? { ref } : {}), adapter: "github" });
    },
  };
}

// Default export for backward compatibility — created lazily with default host
export const githubAdapter: SourceAdapter = {
  name: "github",
  canHandle(source: string): boolean {
    if (source.startsWith("github:")) return true;
    if (LOCAL_PREFIXES.some((p) => source.startsWith(p))) return false;
    if (URL_PROTOCOLS.some((p) => source.startsWith(p))) return false;
    return source.includes("/");
  },
  async resolve(source: string) {
    return createGithubAdapter("https://github.com").resolve(source);
  },
};
```

**File**: `packages/core/src/adapters/resolve.ts`

```typescript
import { loadConfig } from "../config";
import type { ResolvedSource } from "../schemas";
import type { Result } from "../types";
import { err, UserError } from "../types";
import { gitAdapter } from "./git";
import { createGithubAdapter } from "./github";
import { httpAdapter } from "./http";
import { localAdapter } from "./local";
import { npmAdapter } from "./npm";
import type { SourceAdapter } from "./types";

const DEFAULT_GIT_HOST = "https://github.com";

export async function resolveSource(
  source: string,
  gitHost?: string,
): Promise<Result<ResolvedSource, UserError>> {
  const host = gitHost ?? DEFAULT_GIT_HOST;
  const adapters: SourceAdapter[] = [
    gitAdapter,
    npmAdapter,
    httpAdapter,
    localAdapter,
    createGithubAdapter(host),
  ];

  for (const adapter of adapters) {
    if (adapter.canHandle(source)) return adapter.resolve(source);
  }
  return err(
    new UserError(
      `Cannot resolve source: "${source}"`,
      `Try a full URL, GitHub shorthand (user/repo), local path (./path), or a skill name from a configured tap`,
    ),
  );
}
```

**Implementation Notes**:
- `createGithubAdapter(host)` is a factory function — follows the project's factory pattern (see `createCliAdapter` in agents).
- `resolveSource()` gains an optional `gitHost` parameter. Callers that have the config loaded pass `config.default_git_host`; callers that don't get the default.
- The static `githubAdapter` export stays for backward compatibility (tests, direct imports). It always uses `https://github.com`.
- No config loading inside the adapter — the caller passes the host value. This keeps adapters pure (no I/O) per Ports & Adapters.

**Acceptance Criteria**:
- [ ] `resolveSource("owner/repo")` resolves to `https://github.com/owner/repo.git` by default
- [ ] `resolveSource("owner/repo", "https://gitea.example.com")` resolves to `https://gitea.example.com/owner/repo.git`
- [ ] `github:owner/repo` still works (prefix is explicit, uses configured host)
- [ ] Full URLs (`https://...`, `git@...`) are unaffected — handled by `gitAdapter` first
- [ ] Existing tests pass without modification

---

### Unit 6: Update `parseGitHubTapShorthand()` to accept git host

**File**: `packages/core/src/taps.ts`

```typescript
// Change signature:
export function parseGitHubTapShorthand(
  source: string,
  gitHost = "https://github.com",
): GitHubTapShorthand | null {
  const host = gitHost.replace(/\/$/, "");
  let s = source;
  if (s.startsWith("github:")) s = s.slice("github:".length);
  else if (!s.includes("/")) return null;

  if (GH_URL_PROTOCOLS.some((p) => s.startsWith(p))) return null;
  if (GH_LOCAL_PREFIXES.some((p) => s.startsWith(p))) return null;

  const atIdx = s.lastIndexOf("@");
  if (atIdx !== -1) s = s.slice(0, atIdx);

  const parts = s.split("/").filter(Boolean);
  if (parts.length !== 2) return null;

  const [owner, repo] = parts;
  return {
    name: repo!,
    url: `${host}/${owner}/${repo}.git`,
  };
}
```

**Implementation Notes**:
- Default parameter preserves backward compatibility — existing callers without config don't break.
- CLI callers (`tap add` command) should pass `config.default_git_host` from the loaded config.

**Acceptance Criteria**:
- [ ] `parseGitHubTapShorthand("owner/repo")` → `https://github.com/owner/repo.git` (default)
- [ ] `parseGitHubTapShorthand("owner/repo", "https://gitea.example.com")` → `https://gitea.example.com/owner/repo.git`
- [ ] Existing callers without the second arg continue to work

---

### Unit 7: Thread `default_git_host` through CLI commands

**Files**: `packages/cli/src/commands/install.ts`, `packages/cli/src/commands/update.ts`, `packages/cli/src/commands/tap/add.ts`

These commands load config early. After loading, pass `config.default_git_host` to:
- `resolveSource(source, config.default_git_host)` in install/update
- `parseGitHubTapShorthand(source, config.default_git_host)` in tap add

```typescript
// Example change in install.ts (and similarly in update.ts):
// Before:
const resolved = await resolveSource(source);
// After:
const resolved = await resolveSource(source, config.default_git_host);
```

```typescript
// Example change in tap/add.ts:
// Before:
const shorthand = parseGitHubTapShorthand(url);
// After:
const shorthand = parseGitHubTapShorthand(url, config.default_git_host);
```

**Implementation Notes**:
- Config is already loaded at the top of each command handler. This is just threading the value through.
- Search all call sites of `resolveSource` and `parseGitHubTapShorthand` in `packages/cli/src/` to ensure none are missed.

**Acceptance Criteria**:
- [ ] All `resolveSource()` call sites in CLI commands pass `config.default_git_host`
- [ ] All `parseGitHubTapShorthand()` call sites in CLI commands pass `config.default_git_host`
- [ ] No `resolveSource()` or `parseGitHubTapShorthand()` call in CLI uses the default host when config is available

---

## Implementation Order

1. **Unit 1**: `lsRemoteTags()` in `git.ts` — foundation, no dependencies
2. **Unit 4**: `default_git_host` config schema + config-keys — foundation, no dependencies
3. **Unit 2**: Replace `fetchLatestVersion()` — depends on Unit 1
4. **Unit 3**: Replace `downloadAndInstall()` — independent of other units
5. **Unit 5**: Update GitHub adapter + `resolveSource()` — depends on Unit 4
6. **Unit 6**: Update `parseGitHubTapShorthand()` — depends on Unit 4
7. **Unit 7**: Thread config through CLI commands — depends on Units 4, 5, 6

Units 1 and 4 can be implemented in parallel. Units 2, 3, 5, 6 can be implemented in parallel after their dependencies. Unit 7 is last.

---

## Testing

### Unit Tests: `packages/core/src/git.test.ts`

```typescript
describe("lsRemoteTags", () => {
  // Uses a real git repo (test fixture) for integration test
  test("lists tags from a local bare repo", async () => {
    // Create a local bare repo with tags v1.0.0, v2.0.0
    const result = await lsRemoteTags(bareRepoPath, "v*");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toContain("v1.0.0");
    expect(result.value).toContain("v2.0.0");
  });

  test("returns empty array for repo with no matching tags", async () => {
    const result = await lsRemoteTags(bareRepoPath, "nonexistent-*");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual([]);
  });

  test("returns GitError for invalid URL", async () => {
    const result = await lsRemoteTags("https://invalid.example.com/no-repo.git");
    expect(result.ok).toBe(false);
  });
});
```

### Unit Tests: `packages/core/src/self-update.test.ts`

```typescript
describe("fetchLatestVersion (git ls-remote)", () => {
  test("returns highest semver from tags", async () => {
    const mockLsRemote = async () => ok(["v0.9.0", "v1.2.0", "v1.0.0"]);
    const result = await fetchLatestVersion(mockLsRemote);
    expect(result).toBe("1.2.0");
  });

  test("returns null when ls-remote fails", async () => {
    const mockLsRemote = async () => err(new GitError("failed"));
    const result = await fetchLatestVersion(mockLsRemote);
    expect(result).toBeNull();
  });

  test("returns null when no v* tags exist", async () => {
    const mockLsRemote = async () => ok([]);
    const result = await fetchLatestVersion(mockLsRemote);
    expect(result).toBeNull();
  });

  test("ignores malformed tags", async () => {
    const mockLsRemote = async () => ok(["v1.0.0", "release-candidate", "v2.0.0"]);
    const result = await fetchLatestVersion(mockLsRemote);
    expect(result).toBe("2.0.0");
  });
});

describe("downloadAndInstall with gh fallback", () => {
  test("uses gh when available", async () => {
    let fetchCalled = false;
    const mockFetch = async () => { fetchCalled = true; return new Response(null, { status: 200 }); };
    const mockGh = async () => {
      await Bun.write(execPath + ".update", fakeBinary);
      return ok(undefined);
    };
    const result = await downloadAndInstall("9.9.9", mockFetch, execPath, mockGh);
    expect(result.ok).toBe(true);
    expect(fetchCalled).toBe(false);
  });

  test("falls back to fetch when gh fails", async () => {
    const mockGh = async () => err(new UserError("gh not found"));
    const result = await downloadAndInstall("9.9.9", okFetch, execPath, mockGh);
    expect(result.ok).toBe(true);
  });
});
```

### Unit Tests: `packages/core/src/adapters/github.test.ts`

```typescript
describe("createGithubAdapter", () => {
  test("resolves with custom host", async () => {
    const adapter = createGithubAdapter("https://gitea.example.com");
    const result = await adapter.resolve("owner/repo");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.url).toBe("https://gitea.example.com/owner/repo.git");
  });

  test("strips trailing slash from host", async () => {
    const adapter = createGithubAdapter("https://gitea.example.com/");
    const result = await adapter.resolve("owner/repo");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.url).toBe("https://gitea.example.com/owner/repo.git");
  });
});
```

### Unit Tests: `packages/core/src/schemas/config.test.ts`

```typescript
test("default_git_host defaults to https://github.com", () => {
  const result = ConfigSchema.safeParse({});
  expect(result.success).toBe(true);
  if (!result.success) return;
  expect(result.data.default_git_host).toBe("https://github.com");
});

test("default_git_host accepts custom URL", () => {
  const result = ConfigSchema.safeParse({ default_git_host: "https://gitea.example.com" });
  expect(result.success).toBe(true);
  if (!result.success) return;
  expect(result.data.default_git_host).toBe("https://gitea.example.com");
});
```

---

## Verification Checklist

```bash
# 1. All existing tests pass
bun test

# 2. New git ls-remote tests pass
bun test packages/core/src/git.test.ts

# 3. Self-update tests pass with new injectable deps
bun test packages/core/src/self-update.test.ts

# 4. Config schema tests pass
bun test packages/core/src/schemas/config.test.ts

# 5. Adapter tests pass
bun test packages/core/src/adapters/

# 6. No direct fetch() to api.github.com remains
# (grep should return zero matches)
rg "api\.github\.com" packages/

# 7. Build succeeds
bun run build

# 8. Manual smoke test: version check uses git
bun run dev -- doctor
```
