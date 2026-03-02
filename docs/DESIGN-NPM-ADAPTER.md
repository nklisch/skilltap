# Design: npm Source Adapter

Adds `npm:` as a source prefix, letting skilltap install skills published as npm packages. This is the single highest-leverage v0.2 feature — it opens access to the 69K+ skills already on npm via skills.sh, vibe-rules, skills-npm, and other tools that converged on npm as the distribution channel.

## Motivation

By early 2026, at least six independent tools converged on npm for skill distribution:

- **Vercel skills.sh** (69K+ skills, `npx skills`)
- **vibe-rules** (npm `./llms` export convention, used by TanStack)
- **skills-npm** (antfu's `skills/*/SKILL.md` convention)
- **skillpm** (~630 lines, thin npm orchestration)
- **openskills** (global npm install)
- **skild.sh** ("npm for agent skills")

skilltap's git-first model is a strength for private repos and enterprise, but without npm support it can't access the largest existing skill catalog. The npm adapter adds interop without replacing git as the primary channel.

## Source Format

```
npm:<package-name>
npm:<package-name>@<version>
npm:@<scope>/<name>
npm:@<scope>/<name>@<version>
```

Examples:

```bash
skilltap install npm:commit-helper-skill
skilltap install npm:@acme/code-review@1.2.0
skilltap install npm:@vercel/next-skill@latest
```

## Source Resolution

The npm adapter slots into the existing `ADAPTERS[]` array in `core/src/adapters/resolve.ts`. It must be checked **before** the GitHub shorthand adapter (which catches anything with `/`), but **after** URL protocols and local paths.

**Updated resolution order:**

1. URL protocols (`https://`, `http://`, `git@`, `ssh://`) → git adapter
2. `github:` prefix → github adapter
3. `npm:` prefix → **npm adapter** (new)
4. Local paths (`./`, `/`, `~/`) → local adapter
5. Bare `owner/repo` → github adapter (shorthand)
6. Bare name → tap name resolution (in `install.ts`)

### `canHandle()`

```typescript
canHandle(source: string): boolean {
  return source.startsWith("npm:");
}
```

### `resolve()`

1. Strip `npm:` prefix
2. Parse package name and optional `@version` suffix
   - Scoped: `@scope/name` — the `@` is part of the scope, version `@` is the last one
   - Unscoped: `name@version`
   - No version: defaults to `latest`
3. Fetch package metadata from npm registry
4. Return `ResolvedSource` with the tarball URL

```typescript
resolve(source: string): Promise<Result<ResolvedSource, UserError>>
```

Returns:

```typescript
{
  url: "https://registry.npmjs.org/@acme/code-review/-/code-review-1.2.0.tgz",
  ref: "1.2.0",           // resolved version
  adapter: "npm",
}
```

## npm Registry API

All requests go to `https://registry.npmjs.org` (no auth required for public packages).

### Package Metadata

```
GET https://registry.npmjs.org/<package-name>
```

Response (relevant fields):

```json
{
  "name": "@acme/code-review",
  "description": "Thorough code review skill",
  "dist-tags": { "latest": "1.2.0" },
  "versions": {
    "1.2.0": {
      "dist": {
        "tarball": "https://registry.npmjs.org/@acme/code-review/-/code-review-1.2.0.tgz",
        "integrity": "sha512-...",
        "attestations": { "url": "...", "provenance": { ... } }
      }
    }
  }
}
```

### Version Resolution

| Input | Behavior |
|---|---|
| `npm:pkg` | Resolve `dist-tags.latest` |
| `npm:pkg@1.2.0` | Exact version lookup in `versions` |
| `npm:pkg@latest` | Same as no version |
| `npm:pkg@^1.0.0` | **Not supported.** Exact versions or dist-tags only. Semver ranges add complexity for minimal benefit — skills aren't dependencies. |

If the requested version doesn't exist, error:

```
error: Version '2.0.0' not found for npm package '@acme/code-review'.
  Available: 1.2.0, 1.1.0, 1.0.0

  hint: Use 'npm:@acme/code-review' for the latest version (1.2.0).
```

### Private Registries

For private npm registries (Artifactory, Verdaccio, GitHub Packages), respect the user's `.npmrc` configuration. Read the registry URL from:

1. `NPM_CONFIG_REGISTRY` env var
2. Project `.npmrc` (`registry=https://...`)
3. User `~/.npmrc`
4. Default: `https://registry.npmjs.org`

Auth tokens in `.npmrc` are passed through automatically (same as `npm install` does).

## Install Flow

The npm adapter replaces the git clone step with a tarball download + extract:

```
1. resolveSource("npm:@acme/code-review@1.2.0")
   → fetch metadata from registry
   → resolve version → get tarball URL
2. Download tarball to temp dir
3. Extract tarball (tar -xzf)
   → npm tarballs extract to a `package/` directory
4. Scan for SKILL.md files (existing scanner)
5. [rest of install flow unchanged: skill selection, scope, security scan, place, record]
```

### Tarball Handling

npm tarballs are gzipped tars that always extract to a `package/` directory:

```
package/
  package.json
  SKILL.md            ← standalone skill
  skills/
    skill-a/
      SKILL.md        ← multi-skill (antfu convention)
    skill-b/
      SKILL.md
```

After extraction, the scanner runs on `{tempDir}/package/` — the same scanning algorithm used for git clones.

### SKILL.md Discovery in npm Packages

The scanner already handles all relevant layouts. npm packages may use:

1. **Root SKILL.md** — `package/SKILL.md` → standalone skill
2. **Standard path** — `package/.agents/skills/*/SKILL.md` → multi-skill
3. **skills/ convention** — `package/skills/*/SKILL.md` → multi-skill (antfu/skillpm convention)

For case 3, the scanner's existing deep scan (`**/SKILL.md`) catches this without changes. However, to avoid the confirmation prompt for deep scan on npm packages, extend the scanner to also check `skills/*/SKILL.md` as a priority path (between step 2 and step 3 of the scanning algorithm).

### installed.json Record

```json
{
  "name": "code-review",
  "description": "Thorough code review skill",
  "repo": "npm:@acme/code-review",
  "ref": "1.2.0",
  "sha": null,
  "scope": "global",
  "path": null,
  "tap": null,
  "also": ["claude-code"],
  "installedAt": "2026-03-01T12:00:00Z",
  "updatedAt": "2026-03-01T12:00:00Z"
}
```

The `repo` field stores `npm:<package>` (not the tarball URL). This is used by `update` to re-resolve from the registry.

### Updates

`skilltap update` for npm-sourced skills:

1. Read `repo` field → detect `npm:` prefix
2. Fetch current metadata from registry
3. Compare installed `ref` (version) to `dist-tags.latest`
4. If different version available:
   - Download new tarball
   - Diff against installed skill directory (file-level comparison, no git diff)
   - Run security scan on changed files
   - Replace skill directory
   - Update record with new `ref`

Since npm packages aren't git repos, diff-based scanning uses direct file comparison rather than `git diff`. Compare file lists and content between installed and new versions.

## `skilltap find --npm`

Search the npm registry for skill packages.

```bash
skilltap find --npm review
skilltap find --npm code-review --json
```

### Search API

```
GET https://registry.npmjs.org/-/v1/search?text=keywords:agent-skill+review&size=20
```

The npm search API supports `keywords:` filtering. Skill packages should use the `agent-skill` keyword in their `package.json`.

Response (relevant fields):

```json
{
  "objects": [
    {
      "package": {
        "name": "@acme/code-review",
        "version": "1.2.0",
        "description": "Thorough code review skill",
        "keywords": ["agent-skill", "review", "security"],
        "publisher": { "username": "acme", "email": "..." },
        "links": { "npm": "https://www.npmjs.com/package/@acme/code-review" }
      },
      "score": {
        "final": 0.85,
        "detail": { "quality": 0.9, "popularity": 0.7, "maintenance": 0.95 }
      }
    }
  ],
  "total": 42
}
```

### Output

```
$ skilltap find --npm review

  @acme/code-review    1.2.0   Thorough code review skill         [npm]
  review-helper        0.3.1   Quick PR review checklist           [npm]

$ skilltap find --npm review --json
[{"name":"@acme/code-review","version":"1.2.0","description":"...","source":"npm"}]
```

### Integration with Tap Search

`skilltap find review` (without `--npm`) searches taps only — existing behavior unchanged. The `--npm` flag explicitly opts into npm registry search. A future enhancement could search both simultaneously.

## Tap References to npm Sources

Taps can reference npm packages as skill sources:

```json
{
  "name": "my tap",
  "skills": [
    {
      "name": "code-review",
      "description": "Thorough code review",
      "repo": "npm:@acme/code-review",
      "tags": ["review"]
    }
  ]
}
```

When `repo` starts with `npm:`, the install flow uses the npm adapter instead of git clone. This lets tap curators index npm-published skills alongside git-hosted ones.

## Schema Changes

### ResolvedSource

No changes needed. The existing schema already supports arbitrary `adapter` string values:

```typescript
const ResolvedSourceSchema = z.object({
  url: z.string(),       // tarball URL for npm
  ref: z.string().optional(),  // resolved version
  adapter: z.string(),   // "npm"
})
```

### InstalledSkill

No schema changes needed. The `repo` field already accepts any string, and `sha` is already nullable.

### TapSkill

No changes needed. The `repo` field already accepts any string — `npm:@scope/name` is a valid value.

## New Files

```
packages/core/src/adapters/npm.ts     # npm source adapter
packages/core/src/npm-registry.ts     # npm registry API client (fetch metadata, search)
```

### `npm-registry.ts`

Standalone module for npm registry interaction. Keeps HTTP concerns out of the adapter:

```typescript
// Fetch package metadata (all versions)
fetchPackageMetadata(name: string, registryUrl?: string): Promise<Result<NpmPackageMetadata, NetworkError>>

// Resolve a specific version (exact or dist-tag)
resolveVersion(metadata: NpmPackageMetadata, version: string): Result<NpmVersionInfo, UserError>

// Search for packages by keyword
searchPackages(query: string, options?: { keywords?: string[], size?: number }): Promise<Result<NpmSearchResult[], NetworkError>>

// Download and extract a tarball to a temp directory
downloadAndExtract(tarballUrl: string, dest: string, integrity?: string): Promise<Result<string, NetworkError>>
```

Types:

```typescript
interface NpmPackageMetadata {
  name: string;
  description: string;
  distTags: Record<string, string>;
  versions: Record<string, NpmVersionInfo>;
}

interface NpmVersionInfo {
  version: string;
  dist: {
    tarball: string;
    integrity: string;
    attestations?: { url: string; provenance: unknown };
  };
}

interface NpmSearchResult {
  name: string;
  version: string;
  description: string;
  keywords: string[];
  publisher: string;
  score: number;
}
```

## Integrity Verification

npm tarballs include an `integrity` field (SHA-512 hash in SRI format). After downloading, verify the hash matches:

```typescript
const hash = Bun.CryptoHasher.hash("sha512", tarballBuffer);
const expected = integrity.replace("sha512-", "");
if (toBase64(hash) !== expected) {
  return err(new NetworkError("Tarball integrity check failed"));
}
```

This catches corrupted downloads and basic MITM attacks. For stronger guarantees, see [DESIGN-TRUST.md](./DESIGN-TRUST.md) (provenance verification).

## CLI Changes

### `install` command

No changes to flags. The `source` argument now accepts `npm:` prefix — the adapter handles it transparently.

```bash
# All equivalent in terms of the install flow after resolution
skilltap install https://github.com/acme/code-review
skilltap install acme/code-review
skilltap install npm:@acme/code-review
```

### `find` command

New `--npm` flag:

```
--npm              Search npm registry instead of taps
```

### `info` command

When showing info for an npm-sourced skill:

```
$ skilltap info code-review

  code-review (installed, global)
    Thorough code review skill
    Source: npm:@acme/code-review
    Version: 1.2.0
    Also:   claude-code
    Size:   8.2 KB (2 files)
    Installed: 2026-03-01
    Updated:   2026-03-01
```

## Error Conditions

| Condition | Message |
|---|---|
| Package not found | `error: npm package '@acme/nonexistent' not found on registry.` |
| Version not found | `error: Version '2.0.0' not found for npm package '@acme/code-review'. Available: 1.2.0, 1.1.0` |
| Network error | `error: Could not reach npm registry. Check your connection.` |
| No SKILL.md in package | `error: No SKILL.md found in npm package '@acme/code-review'. This package doesn't contain any skills.` |
| Integrity mismatch | `error: Tarball integrity check failed for '@acme/code-review@1.2.0'. The download may be corrupted.` |
| Private package (no auth) | `error: Authentication required for npm package '@acme/private'. Check your .npmrc configuration.` |

## Testing

- **Unit tests**: adapter `canHandle()` and `resolve()` with various source formats
- **Unit tests**: version parsing (scoped, unscoped, with/without version)
- **Unit tests**: tarball extraction and SKILL.md discovery
- **Integration test**: install from a real npm package (or local Verdaccio)
- **Integration test**: `find --npm` search
- **Test fixture**: npm package tarball with known structure (pre-built `.tgz` in test-utils)
