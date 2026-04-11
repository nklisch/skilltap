---
description: The tap.json schema for skill registries. Lists skills with names, descriptions, tags, source repositories, and trust metadata for distribution and discovery.
---

# tap.json Format

A **tap** is a git repository that acts as a skill registry. It contains a `tap.json` file listing skills with their names, descriptions, source repositories, and tags. Taps are how users discover and share collections of skills -- like Homebrew taps, but for agent skills.

skilltap also supports [Claude Code marketplace repos](#claude-code-marketplace-repos) as taps -- if a repo has `.claude-plugin/marketplace.json` instead of `tap.json`, skilltap automatically adapts it.

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
| `plugins` | array | No (default: `[]`) | List of plugin entries |

### Skill Entry Fields

Each entry in the `skills` array:

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | Yes | -- | Skill name (matches the `name` field in SKILL.md frontmatter) |
| `description` | string | Yes | -- | What the skill does |
| `repo` | string | Yes | -- | Source of the skill. Git URL, `github:owner/repo`, or `npm:package-name`. |
| `tags` | array of strings | No | `[]` | Searchable tags for categorization |

### Plugin Entry Fields

Each entry in the `plugins` array describes a full plugin — a bundle of skills, MCP servers, and agent definitions distributed together.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | Yes | -- | Plugin name, used as the install identifier |
| `description` | string | No | `""` | What the plugin does |
| `version` | string | No | -- | Optional version string |
| `tags` | array of strings | No | `[]` | Searchable tags |
| `skills` | array | No | `[]` | Skill entries bundled with this plugin |
| `mcpServers` | string or object | No | -- | Path to `.mcp.json` within the tap repo, or inline object in `.mcp.json` `mcpServers` format |
| `agents` | array | No | `[]` | Agent definition files to install |

Install a tap plugin with:

```bash
skilltap install tap-name/plugin-name
```

Tap plugins appear in `skilltap find` results with a `[plugin]` badge.

### Plugin Skill Entry

Each entry in a plugin's `skills` array:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Skill name (matches SKILL.md frontmatter `name`) |
| `description` | string | No | Short description |
| `path` | string | Yes | Relative path within the tap repo to the skill directory |

### Plugin Agent Entry

Each entry in a plugin's `agents` array:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Agent identifier (e.g. `claude-code`) |
| `path` | string | Yes | Relative path to the agent `.md` file within the tap repo |

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
  ],
  "plugins": [
    {
      "name": "dev-assistant",
      "description": "Full development assistant with skills, MCP filesystem access, and agent config",
      "repo": "https://gitea.example.com/nathan/dev-assistant",
      "tags": ["productivity", "filesystem"],
      "skills": [
        { "name": "dev-assistant", "description": "Development task assistant" }
      ],
      "mcpServers": {
        "filesystem": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/dev"] }
      },
      "agents": [
        { "name": "claude-code", "path": "agent/claude-code.md" }
      ]
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

## Claude Code Marketplace Repos

Claude Code plugin marketplaces use `.claude-plugin/marketplace.json` instead of `tap.json`. skilltap recognizes this format automatically — when you `skilltap tap add` a repo that has `marketplace.json` (and no `tap.json`), it adapts the marketplace data into skilltap's internal tap format.

```bash
# Add the official Anthropic skills marketplace as a tap
skilltap tap add anthropics/skills
```

### How It Works

- If `tap.json` exists, it is used (always takes precedence)
- If only `.claude-plugin/marketplace.json` exists, skilltap parses it and converts each plugin entry to a skill entry
- Plugin sources (GitHub, npm, git URL, relative path) are mapped to the `repo` field that skilltap's install flow understands
- Plugin-only features (MCP servers, LSP servers, hooks, agents) are silently ignored — skilltap only installs SKILL.md content

### Source Mapping

| Marketplace source type | skilltap repo mapping |
|---|---|
| Relative path (`"./plugins/my-plugin"`) | The marketplace repo's own URL |
| `github` (`{ "source": "github", "repo": "owner/repo" }`) | `owner/repo` |
| `url` (`{ "source": "url", "url": "https://..." }`) | The URL directly |
| `git-subdir` (`{ "source": "git-subdir", "url": "..." }`) | The URL (subdirectory path is not preserved) |
| `npm` (`{ "source": "npm", "package": "@org/pkg" }`) | `npm:@org/pkg` |

### Full Plugin Detection

Marketplace plugins whose source uses a relative path (e.g. `"./plugins/my-plugin"`) point to a subdirectory within the marketplace repo itself. When cloning these, skilltap checks for `.claude-plugin/plugin.json` inside the subdirectory. If found, the entry is treated as a full plugin — skills, MCP servers, and agent definitions are all installed. Use `skilltap plugin` to manage these after install.

Plugins sourced from external repos (GitHub, npm, etc.) go through the same auto-detection during install: if the cloned repo contains `.claude-plugin/plugin.json` or `.codex-plugin/plugin.json`, skilltap prompts to install as a full plugin.

### Limitations

- **No namespacing**: Claude Code plugins install skills as `/plugin-name:skill-name`. skilltap installs skills as `/skill-name` (agent-agnostic, no namespace).
- **git-subdir**: The subdirectory path within the source repo is not preserved — skilltap clones the full repo and scans for SKILL.md files.
