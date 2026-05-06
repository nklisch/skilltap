---
description: Taps are curated skill registries — git repos or HTTP endpoints listing skills. Create your own, add a friend's, or subscribe to a community tap. Add, list, update, and search across all of them.
---

# Taps

A **tap** is either a git repository with a `tap.json` file, an HTTP registry endpoint, or a [Claude Code marketplace repo](/reference/tap-format#claude-code-marketplace-repos) (`.claude-plugin/marketplace.json`) that lists skills. Think of taps as curated indexes — they don't contain skills themselves, they point to where skills live.

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

Shows all registered taps with their names, URLs, and skill count. (Pre-v2.0, the type column also distinguished `git` from `http` taps; HTTP support was removed in v2.0.)

## Inspecting a tap

```bash
skilltap tap info <name>
```

Shows the tap's URL, local clone path, when it was last fetched, and how many skills it contains. Useful for diagnosing stale clones or verifying the URL that will be used when cloning.

```bash
skilltap tap info home --json   # machine-readable
```

## Updating taps

```bash
skilltap tap update             # all taps
skilltap tap update home        # one tap
```

Tap indexes are refreshed automatically when you run `skilltap update`. It pulls the latest `tap.json` for every git tap before checking your installed skills — so newly added skills in a tap become discoverable in the same step as your skill updates.

HTTP taps are always live and need no refresh step.

`tap update` is self-healing: if the local clone is missing it re-clones automatically, and it syncs the remote URL from config before pulling. This means you can fix a URL in `config.toml` and run `skilltap tap update <name>` — no manual directory deletion needed.

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

## Browsing and installing from taps

Run `skilltap tap install` to open an interactive picker over all your configured tap skills:

```bash
skilltap tap install
```

Use the search box to filter, Space to toggle selection, Enter to confirm. Skills you've already installed are pre-selected and shown with an `installed` tag — deselecting them will remove them.

To limit the picker to a single tap:

```bash
skilltap tap install --tap home
```

## Installing by name

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

### Tap plugins

Taps can also distribute full plugins — bundles that include skills, MCP server entries, and agent definition files — via a `plugins` array in `tap.json`:

```json
{
  "name": "my-tap",
  "skills": [...],
  "plugins": [
    {
      "name": "dev-assistant",
      "description": "Development assistant with filesystem MCP access",
      "repo": "https://github.com/user/dev-assistant",
      "tags": ["productivity", "filesystem"]
    }
  ]
}
```

Plugins appear in `skilltap find` results with a `[plugin]` badge alongside regular skills. Install a tap plugin with:

```bash
skilltap install my-tap/dev-assistant
```

This installs all plugin components — skills, MCP servers, and agent files — in a single step. Use `skilltap plugin` to manage installed plugins.

See the [tap.json format reference](/reference/tap-format#plugin-entry-fields) for the full plugin entry schema.

### Publish your tap

Push the repository to any git host — GitHub, GitLab, Bitbucket, a self-hosted server. Then share the URL:

```bash
skilltap tap add my-tap https://github.com/you/my-tap
```

Anyone with access to the repo can add it as a tap.

## HTTP registry taps (removed in v2.0)

::: warning Removed in v2.0
HTTP registry taps were removed in v2.0 — taps are now git-only. v0.x configs with `type = "http"` are silently filtered with a one-time stderr warning, and `skilltap migrate` lists them as needing manual conversion or removal. The original design is described in [VISION.md](https://github.com/nklisch/skilltap/blob/main/docs/VISION.md#http-registry-removed-in-v20--historical-reference-only) for historical reference.

If you need a non-git distribution path, host your `tap.json` in a private git repo (Gitea, GitLab self-hosted, or any HTTP-accessible bare repo). `skilltap tap add` works against any git URL.
:::

## Auth errors and URL fallback

When a `git clone` fails due to authentication, skilltap automatically retries with the alternate URL protocol -- HTTPS switches to SSH, and SSH switches to HTTPS. If the fallback succeeds, the working URL is saved to your config so future operations use it directly.

This means you can add a tap with an HTTPS URL, and if your machine only has SSH keys configured (no HTTPS credentials), skilltap will transparently fall back to SSH and remember the SSH URL.

::: tip URL rewrites
Git also applies `url.insteadOf` rewrites transparently before connecting, so the actual connection may use a different URL than what you configured. Use `skilltap tap info <name>` to see what URL is stored.
:::

For repos where neither HTTPS nor SSH can succeed through skilltap (e.g. complex auth, custom URL schemes, VPN-only servers), use `skilltap link` to symlink a local clone instead:

```bash
# Clone manually however your auth requires
git clone git@internal.corp:team/my-skill ~/dev/my-skill

# Then link the local clone — no cloning through skilltap
skilltap link ~/dev/my-skill
```
