---
description: The tap.json schema for skill registries. Lists skills with names, descriptions, tags, source repositories, and trust metadata for distribution and discovery.
---

# tap.json Format

A **tap** is a git repository that acts as a skill registry. It contains a `tap.json` file listing skills with their names, descriptions, source repositories, and tags. Taps are how users discover and share collections of skills -- like Homebrew taps, but for agent skills.

## What a Tap Does

- Provides a searchable index of skills across repositories
- Enables `skilltap install <name>` without remembering full git URLs
- Powers `skilltap find` search across all configured taps
- Can be public or private (any git host works)

## Schema

The `tap.json` file must be at the root of the tap repository.

### Top-Level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Human-readable name for the tap |
| `description` | string | No | What this tap collection is about |
| `skills` | array | Yes | List of skill entries |

### Skill Entry Fields

Each entry in the `skills` array:

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | Yes | -- | Skill name (matches the `name` field in SKILL.md frontmatter) |
| `description` | string | Yes | -- | What the skill does |
| `repo` | string | Yes | -- | Source of the skill. Git URL, `github:owner/repo`, or `npm:package-name`. |
| `tags` | array of strings | No | `[]` | Searchable tags for categorization |

## Full Example

```json
{
  "name": "nathan's skills",
  "description": "Curated collection of development skills",
  "skills": [
    {
      "name": "commit-helper",
      "description": "Generates conventional commit messages from staged changes",
      "repo": "https://gitea.example.com/nathan/commit-helper",
      "tags": ["git", "productivity", "commits"]
    },
    {
      "name": "code-review",
      "description": "Thorough code review with security focus",
      "repo": "https://gitea.example.com/nathan/code-review",
      "tags": ["review", "security", "quality"]
    },
    {
      "name": "termtube-dev",
      "description": "Development workflow for the termtube project",
      "repo": "https://gitea.example.com/nathan/termtube",
      "tags": ["termtube", "workflow"]
    }
  ]
}
```

## Creating a Tap

Use `skilltap tap init` to scaffold a new tap:

```bash
skilltap tap init my-tap
```

This creates:

```
my-tap/
  tap.json      # Empty skills array, ready to edit
  .git/         # Initialized git repo
```

Edit `tap.json` to add your skills, then push:

```bash
cd my-tap
# Edit tap.json...
git add tap.json
git commit -m "Add skills"
git remote add origin https://gitea.example.com/user/my-tap
git push -u origin main
```

Others can then add your tap:

```bash
skilltap tap add friend https://gitea.example.com/user/my-tap
```

## Where Taps Are Stored

When a user adds a tap with `skilltap tap add`, the tap repo is cloned to:

```
~/.config/skilltap/taps/{name}/
```

For example:

```
~/.config/skilltap/taps/
  home/
    tap.json
    .git/
  community/
    tap.json
    .git/
```

Tap entries are also recorded in `~/.config/skilltap/config.toml`:

```toml
[[taps]]
name = "home"
url = "https://gitea.example.com/nathan/my-skills-tap"

[[taps]]
name = "community"
url = "https://github.com/someone/awesome-skills-tap"
```

## Validation

`tap.json` is validated against its Zod schema when:

- A tap is added (`skilltap tap add`)
- A tap is updated (via `skilltap update`, which refreshes all git tap repos before updating skills)
- Taps are loaded for search or install resolution

If validation fails, skilltap reports a clear parse error with the specific issue:

```
error: Invalid tap.json in 'https://example.com/bad-tap': skills[2].repo is required
```

Invalid taps are skipped gracefully during `loadTaps()` -- a broken tap does not prevent other taps from working.

## Managing Taps

| Command | Description |
|---------|-------------|
| `skilltap tap add <name> <url>` | Add a tap by cloning its repo |
| `skilltap tap remove <name>` | Remove a tap (does not uninstall skills from it) |
| `skilltap tap list` | List configured taps with skill counts |
| `skilltap tap init <name>` | Scaffold a new tap repo |

## npm Sources in Taps

Taps can reference skills published to the npm registry using an `npm:` prefix in the `repo` field:

```json
{
  "name": "npm skills collection",
  "skills": [
    {
      "name": "vibe-rules",
      "description": "Curated coding rules for AI agents",
      "repo": "npm:vibe-rules",
      "tags": ["rules", "coding"]
    },
    {
      "name": "my-scoped-skill",
      "description": "A skill from a scoped npm package",
      "repo": "npm:@myorg/my-skill",
      "tags": ["internal"]
    }
  ]
}
```

When a user installs a skill whose `repo` starts with `npm:`, the tarball is downloaded from the npm registry rather than cloned from git.

## HTTP Registry API

Instead of a static `tap.json` file in a git repo, you can serve a skill index over HTTP. Any web server that returns the correct JSON responses qualifies as an HTTP registry.

`skilltap tap add` auto-detects HTTP registries by making a `GET /skills?limit=1` probe. If it returns a valid response, the tap is registered as HTTP type.

### Required Endpoint

**`GET /skills`** — Return a list of skills.

Query parameters:

| Parameter | Type | Description |
|-----------|------|-------------|
| `q` | string | Search term. Filter results by name/description/tags. |
| `tag` | string | Filter by a specific tag. |
| `limit` | integer | Max number of results to return. |
| `cursor` | string | Pagination cursor from previous response. |

Response schema:

```json
{
  "skills": [
    {
      "name": "commit-helper",
      "description": "Generates conventional commit messages",
      "source": {
        "type": "git",
        "url": "https://github.com/user/commit-helper"
      },
      "tags": ["git", "productivity"]
    },
    {
      "name": "vibe-rules",
      "description": "Curated coding rules",
      "source": {
        "type": "npm",
        "package": "vibe-rules"
      },
      "tags": ["rules"]
    }
  ],
  "cursor": "next-page-cursor",
  "total": 42
}
```

The `source` field is a discriminated union:

| `type` | Additional fields | Description |
|--------|-------------------|-------------|
| `"git"` | `url` | Git-cloneable URL |
| `"github"` | `repo` (`owner/repo`) | GitHub shorthand |
| `"npm"` | `package` | npm package name (with optional `version`) |
| `"url"` | `url` | Direct tarball download URL |

### Optional Endpoint

**`GET /skills/{name}`** — Return detail for a specific skill.

Returns a single skill object (same shape as a `skills` array entry). Used by `skilltap info` and for install resolution.

### Authentication

HTTP registries can require a bearer token. Users configure credentials in `config.toml`:

```toml
[[taps]]
name = "private-registry"
url = "https://skills.example.com/api/v1"
type = "http"
auth_env = "REGISTRY_TOKEN"
```

skilltap sends `Authorization: Bearer <token>` on all requests to that tap.

### Static Hosting

A static JSON file hosted on any web server works as a read-only registry. The minimum is a single file at `/skills` (or at the configured URL root) that returns the response schema above. No server logic required.

## Multi-Skill Repos in Taps

A single repo can contain multiple skills (see [SKILL.md Format](/reference/skill-format#multi-skill-repo)). In a tap, each skill gets its own entry pointing to the same repo:

```json
{
  "name": "termtube tap",
  "skills": [
    {
      "name": "termtube-dev",
      "description": "Development workflow for termtube",
      "repo": "https://gitea.example.com/nathan/termtube",
      "tags": ["termtube"]
    },
    {
      "name": "termtube-review",
      "description": "Code review checklist for termtube",
      "repo": "https://gitea.example.com/nathan/termtube",
      "tags": ["termtube", "review"]
    }
  ]
}
```

When a user installs either skill by name, skilltap clones the repo and discovers both skills inside it.
