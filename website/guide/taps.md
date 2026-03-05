---
description: Taps are curated skill registries — git repos or HTTP endpoints listing skills. Create your own, add a friend's, or subscribe to a community tap. Add, list, update, and search across all of them.
---

# Taps

A **tap** is either a git repository with a `tap.json` file or an HTTP registry endpoint that lists skills. Think of taps as curated indexes — they don't contain skills themselves, they point to where skills live.

Creating a tap is as simple as a git repo and a JSON file. Anyone can stand one up in minutes and share the URL — no registry account, no approval process.

## Why taps?

Taps solve discovery, curation, and sharing:

- **Host your own catalog** -- create a tap for your team, your friends, your open-source project, or just yourself. A git repo is all it takes.
- **Team and org collections** -- your organization maintains a tap listing approved internal skills; developers install by name, not by URL
- **Community indexes** -- open-source tap repos that catalog useful skills across the ecosystem
- **Personal curation** -- your own tap of skills you use and recommend, shareable with anyone

## Adding a tap

```bash
skilltap tap add <name> <url>
skilltap tap add <owner/repo>
```

skilltap auto-detects whether the URL is an HTTP registry or a git repo. Git taps are cloned to `~/.config/skilltap/taps/<name>/`. HTTP taps are queried live with no local clone.

For GitHub repos, you can use shorthand — the tap name is derived from the repo:

```bash
skilltap tap add nklisch/skilltap-skills    # name → "skilltap-skills"
```

Or specify a custom name with the two-arg form:

```bash
skilltap tap add skilltap https://github.com/nklisch/skilltap-skills
```

## Listing taps

```bash
skilltap tap list
```

Shows all registered taps with their names, URLs, type (`git`/`http`), and skill count.

## Updating taps

Pull the latest changes for all git taps:

```bash
skilltap tap update
```

Or update a specific tap:

```bash
skilltap tap update community
```

HTTP taps are always live — `tap update` is a no-op for them.

## Removing a tap

```bash
skilltap tap remove community
```

This removes the tap index from your system. It does **not** uninstall any skills you installed from that tap -- those remain in place.

## Searching across taps

Find skills by name, description, or tag across all your registered taps:

```bash
skilltap find <query>
```

Multi-word queries work without quoting — `skilltap find git hooks` and `skilltap find "git hooks"` are equivalent.

This prints a table of matching skills with their tap, name, description, and tags. By default, results from the skills.sh public registry are also included (sorted by install count). To search only your local taps:

```bash
skilltap find --local <query>
```

For interactive selection (pick a result and install it directly):

```bash
skilltap find -i
```

If you already have a query, pass it to skip the search prompt:

```bash
skilltap find -i <query>
```

## Installing from a tap

Once you have taps registered, install a skill by name:

```bash
skilltap install research
```

If the name exists in multiple taps, skilltap prompts you to choose which one.

To install a specific git ref (branch, tag, or commit):

```bash
skilltap install research@v1.0.0
```

## Creating your own tap

### Scaffold a new tap

```bash
skilltap tap init my-tap
```

This creates a `my-tap/` directory with an empty `tap.json`.

### The tap.json format

A tap is just a JSON file listing skills:

```json
{
  "name": "my-tap",
  "description": "My curated skills",
  "skills": [
    {
      "name": "commit-helper",
      "description": "Generate conventional commit messages",
      "repo": "https://github.com/user/commit-helper",
      "tags": ["git", "productivity"]
    }
  ]
}
```

Each entry in `skills` has:

| Field         | Required | Description                            |
| ------------- | -------- | -------------------------------------- |
| `name`        | Yes      | Unique skill name within this tap      |
| `description` | Yes      | Short description shown in search      |
| `repo`        | Yes      | Source URL — git URL, `github:owner/repo`, or `npm:package-name` |
| `tags`        | No       | Array of strings for filtering/search  |

### Publish your tap

Push the repository to any git host — GitHub, GitLab, Bitbucket, a self-hosted server. Then share the URL:

```bash
skilltap tap add my-tap https://github.com/you/my-tap
```

Anyone with access to the repo can add it as a tap.

## HTTP registry taps

HTTP taps are live web services that serve skill indexes over an API. They're useful for enterprise registries, large or dynamic catalogs, or anything that doesn't fit cleanly into a git repo.

### Adding an HTTP tap

`skilltap tap add` probes the URL to auto-detect the type:

```bash
skilltap tap add enterprise https://skills.example.com/api/v1
```

If the URL returns a valid skill list response, it's registered as an HTTP tap. Otherwise skilltap falls back to treating it as a git repo.

### HTTP vs git taps

|  | Git tap | HTTP tap |
|---|---|---|
| Storage | Cloned locally | No local clone |
| Updates | `tap update` to pull | Always live |
| Discovery | Reads local `tap.json` | Queries the API |
| Auth | Git credentials | Bearer token |

### Authentication

For private HTTP registries, add credentials in your config:

```toml
[[taps]]
name = "enterprise"
url = "https://skills.example.com/api/v1"
type = "http"
auth_env = "SKILLS_REGISTRY_TOKEN"
```

`auth_env` names an environment variable that holds the bearer token. Prefer it over `auth_token` (which embeds the token directly in config).

See the [tap-format reference](/reference/tap-format#http-registry-api) for the full HTTP registry API spec.
