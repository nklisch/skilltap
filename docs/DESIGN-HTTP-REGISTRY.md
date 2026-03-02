# Design: HTTP Registry Adapter

Adds support for HTTP registries as a tap source type. A registry is any server (or static file host) implementing a small JSON API. This covers enterprise private registries, self-hosted indexes, and future community registries.

VISION.md already sketches the endpoint design. This doc finalizes the spec and defines the adapter integration.

## Motivation

Git taps work well for small curated lists, but they require cloning a repo and reading JSON from disk. HTTP registries serve three use cases git taps can't:

1. **Enterprise/private registries** — Artifactory, Verdaccio, or a company's internal skill index behind SSO. IT can host a registry endpoint without giving every developer clone access to an index repo.
2. **Large indexes** — a tap with 10,000+ skills is unwieldy as a single `tap.json`. An HTTP registry supports pagination and server-side search.
3. **Dynamic registries** — registries that aggregate skills from multiple sources (scraping GitHub, indexing npm, accepting submissions) can serve results via API without maintaining a git repo.

## Tap Type Detection

When a user adds a tap, skilltap auto-detects the type:

```bash
# Git tap (existing behavior)
skilltap tap add home https://github.com/nathan/my-skills-tap

# HTTP registry (new)
skilltap tap add company https://skills.company.com/skilltap/v1

# Explicit override
skilltap tap add company https://skills.company.com/skilltap/v1 --type http
skilltap tap add home https://github.com/nathan/my-skills-tap --type git
```

**Auto-detection algorithm:**

1. If `--type` is specified, use that.
2. Attempt `GET {url}` with `Accept: application/json`.
   - If response is JSON with a `skills` array → HTTP registry.
   - If response is not JSON, or connection refused, or 404 → fall through.
3. Attempt git clone (existing behavior).
   - If clone succeeds and `tap.json` exists → git tap.
4. If both fail → error.

Auto-detection runs only during `tap add`. After that, the type is stored in config and used directly.

### Config Storage

```toml
[[taps]]
name = "home"
url = "https://github.com/nathan/my-skills-tap"
# type defaults to "git" when absent (backward-compatible)

[[taps]]
name = "company"
url = "https://skills.company.com/skilltap/v1"
type = "http"
```

Schema update:

```typescript
// In ConfigSchema, taps array
taps: z.array(z.object({
  name: z.string(),
  url: z.string(),
  type: z.enum(["git", "http"]).default("git"),
})).default([]),
```

## API Specification

A registry implements three endpoints. All responses are JSON with `Content-Type: application/json`.

### `GET /skills`

List and search skills.

**Query parameters:**

| Param | Type | Default | Description |
|---|---|---|---|
| `q` | string | (none) | Search query (matched against name, description, tags) |
| `tag` | string | (none) | Filter by tag. Repeatable. |
| `limit` | integer | 50 | Max results per page (1–100) |
| `cursor` | string | (none) | Pagination cursor from previous response |

**Response:**

```json
{
  "skills": [
    {
      "name": "commit-helper",
      "description": "Generates conventional commit messages",
      "version": "1.2.0",
      "author": "nathan",
      "tags": ["git", "productivity"],
      "source": {
        "type": "git",
        "url": "https://github.com/nathan/commit-helper",
        "ref": "v1.2.0"
      },
      "trust": {
        "verified": true,
        "verifiedBy": "nathan"
      }
    }
  ],
  "total": 42,
  "cursor": "eyJvZmZzZXQiOjUwfQ=="
}
```

**Zod schema:**

```typescript
const RegistrySourceSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("git"), url: z.string(), ref: z.string().optional() }),
  z.object({ type: z.literal("github"), repo: z.string(), ref: z.string().optional() }),
  z.object({ type: z.literal("npm"), package: z.string(), version: z.string().optional() }),
  z.object({ type: z.literal("url"), url: z.string() }),
]);

const RegistrySkillSchema = z.object({
  name: z.string(),
  description: z.string(),
  version: z.string().optional(),
  author: z.string().optional(),
  tags: z.array(z.string()).default([]),
  source: RegistrySourceSchema,
  trust: z.object({
    verified: z.boolean().default(false),
    verifiedBy: z.string().optional(),
  }).optional(),
});

const RegistryListResponseSchema = z.object({
  skills: z.array(RegistrySkillSchema),
  total: z.number().int().optional(),
  cursor: z.string().optional(),
});
```

### `GET /skills/{name}`

Skill detail with version history.

**Response:**

```json
{
  "name": "commit-helper",
  "description": "Generates conventional commit messages",
  "author": "nathan",
  "license": "MIT",
  "tags": ["git", "productivity"],
  "versions": [
    { "version": "1.2.0", "publishedAt": "2026-02-28T12:00:00Z" },
    { "version": "1.1.0", "publishedAt": "2026-01-15T12:00:00Z" }
  ],
  "source": {
    "type": "git",
    "url": "https://github.com/nathan/commit-helper"
  },
  "trust": {
    "verified": true,
    "verifiedBy": "nathan"
  }
}
```

**Zod schema:**

```typescript
const RegistryDetailResponseSchema = z.object({
  name: z.string(),
  description: z.string(),
  author: z.string().optional(),
  license: z.string().optional(),
  tags: z.array(z.string()).default([]),
  versions: z.array(z.object({
    version: z.string(),
    publishedAt: z.string().optional(),
  })).default([]),
  source: RegistrySourceSchema,
  trust: z.object({
    verified: z.boolean().default(false),
    verifiedBy: z.string().optional(),
  }).optional(),
});
```

### `GET /skills/{name}/download`

Download a skill as a tarball.

**Query parameters:**

| Param | Type | Default | Description |
|---|---|---|---|
| `version` | string | latest | Version to download |

**Response:** `200 OK` with `Content-Type: application/gzip` body (`.tar.gz`).

The tarball structure mirrors npm — extracts to a directory containing `SKILL.md` and any supporting files.

**Alternative:** the `source` object in the skill detail may point to a git repo or npm package instead of providing a download endpoint. In that case, skilltap uses the appropriate adapter (git clone or npm tarball) rather than the download endpoint. The download endpoint is optional — registries that aggregate existing sources don't need to host tarballs.

## Install Flow Integration

When a skill name resolves from an HTTP registry tap, the install flow branches based on the `source.type`:

```
1. skilltap install commit-helper
2. Tap resolution finds match in "company" (HTTP registry)
3. Fetch skill detail: GET https://skills.company.com/skilltap/v1/skills/commit-helper
4. Read source:
   a. type: "git"   → resolve as git URL, continue to clone
   b. type: "github" → resolve as GitHub shorthand, continue to clone
   c. type: "npm"   → resolve as npm package, continue to tarball download
   d. type: "url"   → download tarball directly, extract, scan
5. [rest of install flow unchanged]
```

This means an HTTP registry doesn't need to host skill content at all — it can just point to where the skill lives. The download endpoint is a convenience for registries that want to serve content directly (e.g., bundled enterprise skills behind auth).

## Search Integration

### `skilltap find` with HTTP taps

`skilltap find` already searches across all taps. For HTTP registry taps, it hits the search endpoint instead of scanning a local `tap.json`:

```
skilltap find review
  → For each tap:
    - Git tap: search local tap.json (existing)
    - HTTP tap: GET {url}/skills?q=review
  → Merge results, display
```

Results from HTTP registries are tagged with the tap name, same as git tap results:

```
$ skilltap find review

  code-review        Thorough code review              [home]        (git tap)
  security-review    Security-focused review            [company]     (HTTP registry)
```

### Pagination

For interactive mode (`skilltap find -i`), results are fetched page by page as the user scrolls. The `cursor` from each response is passed to the next request.

For non-interactive mode, a single page of results is shown (default 50). No automatic pagination — if the user wants more, they can refine the query.

## Tap Operations

### `tap add` (HTTP)

```bash
$ skilltap tap add company https://skills.company.com/skilltap/v1

Checking registry...
✓ Added tap 'company' (42 skills)
  https://skills.company.com/skilltap/v1
```

Validation: fetch `GET {url}/skills?limit=1` to verify the endpoint responds with valid JSON. If it returns a valid `RegistryListResponseSchema` response, accept. Otherwise error.

No local clone — HTTP taps don't store anything on disk except the config entry.

### `tap remove` (HTTP)

Remove the config entry. No directory cleanup needed.

### `tap update` (HTTP)

No-op for HTTP registries — they're always live. Print a message:

```
$ skilltap tap update company
  company: HTTP registry (always up to date)
```

### `tap list`

Show type:

```
$ skilltap tap list

  home       git    https://github.com/nathan/my-tap               3 skills
  company    http   https://skills.company.com/skilltap/v1          42 skills
```

The skill count for HTTP taps comes from the `total` field in the list response. Fetched live (cached briefly to avoid hammering the endpoint on every `tap list`).

## Authentication

HTTP registries may require authentication. skilltap supports:

| Method | Config | How it works |
|---|---|---|
| None | (default) | Public registry, no auth |
| Bearer token | `auth_token` in tap config | `Authorization: Bearer {token}` header |
| Basic auth | Encoded in URL | `https://user:pass@registry.example.com/...` |

### Config

```toml
[[taps]]
name = "company"
url = "https://skills.company.com/skilltap/v1"
type = "http"
auth_token = "sk-abc123..."
```

Schema update:

```typescript
taps: z.array(z.object({
  name: z.string(),
  url: z.string(),
  type: z.enum(["git", "http"]).default("git"),
  auth_token: z.string().optional(),
})).default([]),
```

The token is sent as a `Bearer` token on all requests to that registry:

```
Authorization: Bearer sk-abc123...
```

**Security note:** tokens in `config.toml` are plaintext. This matches how `.npmrc` stores tokens and how git credential helpers work. A future enhancement could integrate with system keychains, but for v0.2 plaintext in a user-owned config file is acceptable.

Alternatively, the token can come from an environment variable:

```toml
[[taps]]
name = "company"
url = "https://skills.company.com/skilltap/v1"
type = "http"
auth_env = "COMPANY_SKILLS_TOKEN"
```

When `auth_env` is set, skilltap reads the token from `process.env[auth_env]` at request time. This is preferred for CI and shared machines.

## Static Hosting

An HTTP registry can be a directory of static JSON files behind any web server (nginx, S3, GitHub Pages, Cloudflare Pages):

```
registry/
  skilltap/v1/
    skills.json                           → GET /skilltap/v1/skills
    skills/
      commit-helper.json                  → GET /skilltap/v1/skills/commit-helper
      commit-helper/
        commit-helper-1.2.0.tar.gz        → GET .../download?version=1.2.0
      code-review.json
      code-review/
        code-review-2.0.0.tar.gz
```

For static hosting, `skills.json` contains the full list (no pagination — static files can't paginate). The `?q=` search parameter is ignored — client-side filtering handles it. skilltap detects this by the absence of `cursor` in the response and falls back to client-side search.

A generator script can build this structure from a `tap.json`:

```bash
# Future: skilltap tap export --format http --out ./registry
```

## Error Handling

| Condition | Message |
|---|---|
| Registry unreachable | `error: Could not reach registry at 'https://...'. Check your connection.` |
| Invalid response | `error: Registry at 'https://...' returned invalid JSON. Expected skills list.` |
| Auth required (401) | `error: Authentication required for registry 'company'. Set auth_token in config or auth_env for environment variable.` |
| Auth failed (403) | `error: Authentication failed for registry 'company'. Check your token.` |
| Skill not found (404) | `error: Skill 'nonexistent' not found in registry 'company'.` |
| Download failed | `error: Could not download skill from 'https://...'.` |
| Rate limited (429) | `error: Rate limited by registry 'company'. Try again later.` |

## Caching

HTTP responses are cached briefly to avoid redundant requests during a single skilltap invocation:

- Skill list/search results: cached for the duration of the command (in-memory)
- Skill detail: cached for the duration of the command
- Tarballs: not cached (downloaded to temp dir, installed, temp cleaned up)

No persistent disk cache for HTTP responses. Every new invocation hits the registry fresh. This keeps things simple and avoids stale data.

## New Files

```
packages/core/src/registry/
  types.ts            # Registry response schemas
  client.ts           # HTTP client (fetch, auth, error handling)
  index.ts            # barrel export
packages/core/src/adapters/http.ts   # HTTP registry source adapter (for direct URL sources)
```

The registry client is used by `taps.ts` when loading/searching HTTP taps. The `http.ts` adapter handles `type: "url"` sources from registry responses (direct tarball downloads).

## Compatibility with Existing Formats

The registry response `source` object aligns with source types from Claude Code's `marketplace.json`. If a future standard emerges, the registry API can adopt it — the response format is flexible enough to evolve without breaking clients.

The `source.type: "git"` and `source.type: "npm"` values map directly to skilltap's existing source adapters. No translation layer needed — the registry just tells skilltap where to find the skill, and skilltap uses its existing adapters to fetch it.

## Testing

- **Unit tests**: response schema validation (valid, invalid, missing fields)
- **Unit tests**: type auto-detection logic (JSON response vs. non-JSON)
- **Unit tests**: auth header construction (bearer token, env var, none)
- **Integration test**: `tap add` with a mock HTTP server (use Bun.serve in test)
- **Integration test**: `find` across mixed git + HTTP taps
- **Integration test**: install from HTTP registry with `source.type: "git"` (round-trips through git adapter)
- **Integration test**: install from HTTP registry with `source.type: "url"` (direct tarball download)
- **Test fixture**: static registry directory with pre-built JSON files
