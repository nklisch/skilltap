# Taps

A **tap** is a git repository containing a `tap.json` file that lists skills with their names, descriptions, source URLs, and tags. Think of taps as curated indexes -- they don't contain skills themselves, they point to where skills live.

## Why taps?

Taps solve discovery and curation:

- **Team collections** -- your organization maintains a tap listing approved internal skills
- **Community indexes** -- open-source tap repos that catalog useful skills across the ecosystem
- **Personal curation** -- your own tap of skills you use and recommend

## Adding a tap

```bash
skilltap tap add <name> <url>
```

This clones the tap repository to `~/.config/skilltap/taps/<name>/` and makes its skills searchable.

```bash
skilltap tap add community https://github.com/skilltap/community-tap
```

## Listing taps

```bash
skilltap tap list
```

Shows all registered taps with their names and URLs.

## Updating taps

Pull the latest changes for all taps:

```bash
skilltap tap update
```

Or update a specific tap:

```bash
skilltap tap update community
```

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

This prints a table of matching skills with their tap, name, description, and tags.

For interactive selection (pick a result and install it directly):

```bash
skilltap find -i <query>
```

## Installing from a tap

Once you have taps registered, install a skill by name:

```bash
skilltap install commit-helper
```

If the name exists in multiple taps, skilltap prompts you to choose which one.

To install a specific git ref (branch, tag, or commit):

```bash
skilltap install commit-helper@v2.0
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
| `repo`        | Yes      | Git URL where the skill lives          |
| `tags`        | No       | Array of strings for filtering/search  |

### Publish your tap

Push the repository to any git host -- GitHub, GitLab, Bitbucket, a self-hosted server. Then share the URL:

```bash
skilltap tap add my-tap https://github.com/you/my-tap
```

Anyone with access to the repo can add it as a tap.
