---
description: Share and manage agent skills across any group — a company team, an open-source project, or a circle of friends. Set up a tap, share the URL, and everyone stays in sync.
---

# Sharing Skills with a Team or Group

A tap is all you need to share skills with others — whether that's a company engineering team, an open-source project, or a group of friends. It's a git repo on any host you already use. You maintain it, others subscribe, and everyone stays in sync with `skilltap update`.

The same pattern scales from two friends to a thousand developers.

## The shared tap model

The pattern is simple:

1. **You create** a tap repo on any git host (GitHub, GitLab, Gitea, Bitbucket, self-hosted, wherever)
2. **Others add** your tap once with a single command
3. **Everyone installs** skills by name from your catalog — no URLs to copy-paste
4. **When skills change**, subscribers run `skilltap update` to pull the latest and review what changed

No external dependencies, no centralized service, no per-agent configuration.

## Setting up a tap

The steps are the same whether you're setting up a company catalog, an open-source project's shared skills, or a tap to share with friends.

### Create the tap

```bash
skilltap tap init acme-skills
cd acme-skills
```

This creates a directory with an empty `tap.json`. It's just a git repo — initialize it and push it to wherever you host code:

```bash
git init
git add .
git commit -m "Init tap"
git remote add origin https://github.com/you/acme-skills   # or any git host
git push -u origin main
```

Share the URL with anyone you want to subscribe.

### Add skills to the catalog

Edit `tap.json` to list your skills:

```json
{
  "name": "acme-skills",
  "description": "ACME engineering skill catalog",
  "skills": [
    {
      "name": "code-reviewer",
      "description": "Review code for bugs, style, and security issues",
      "repo": "https://gitea.acme.com/eng/skill-code-reviewer",
      "tags": ["review", "quality"]
    },
    {
      "name": "pr-helper",
      "description": "Draft PR descriptions from git diff",
      "repo": "https://gitea.acme.com/eng/skill-pr-helper",
      "tags": ["git", "productivity"]
    },
    {
      "name": "commit-helper",
      "description": "Write conventional commit messages",
      "repo": "https://gitea.acme.com/eng/skill-commit-helper",
      "tags": ["git", "productivity"]
    }
  ]
}
```

Each `repo` can be any URL skilltap supports: a full git URL, a `github:owner/repo` shorthand, or an `npm:package-name` for npm-published skills.

Commit and push whenever you add or update entries.

### Validate skills before listing them

Before adding a skill to your tap, verify it passes validation:

```bash
skilltap verify ./skill-code-reviewer
```

This checks the SKILL.md frontmatter, runs a static security scan, and prints a snippet ready to paste into `tap.json`.

## Subscribing to a tap

Anyone you share the URL with can add your tap with one command:

```bash
skilltap tap add acme https://gitea.acme.com/eng/acme-skills
```

After that, they install and update from your catalog by name — no URLs to copy-paste, no extra setup:

```bash
# Search the catalog
skilltap find

# Install a skill
skilltap install code-reviewer --global --also claude-code
```

For a company or project, put this in your onboarding docs or runbook. For a friend group, just share the one-liner in a message.

## Private git hosts

Skills can live on private repos. Because skilltap uses `git clone` under the hood, **your existing SSH keys and credential helpers just work** — no additional authentication to configure.

For SSH:
```bash
skilltap tap add acme git@gitea.acme.com:eng/acme-skills.git
```

For HTTPS with a credential helper (like `gh auth`, `git-credential-manager`, or `.netrc`):
```bash
skilltap tap add acme https://gitea.acme.com/eng/acme-skills
```

If your credential helper is already configured for `git clone`, skilltap requires no additional setup.

## Keeping skills up to date

When skills are updated in the tap — bug fix, improved prompt, new capability — subscribers pull the latest with:

```bash
skilltap update
```

This fetches each installed skill, diffs only the changed lines, re-runs the scan on the diff, and applies the update. Developers see what changed before it lands.

For CI/CD or automated environments:

```bash
skilltap update --yes
```

## Project manifest (v2.0 — pinned dependencies)

For teams that want the project to *declare* its skill dependencies (not just rely on the tap catalog), v2.0 added a Cargo-style manifest + lockfile workflow:

- **`skilltap.toml`** at the project root lists which skills + plugins the project uses
- **`skilltap.lock`** captures exact ref + sha for each entry (auto-managed)
- Teammates check out the project and run `skilltap sync --apply` to install the exact pinned versions

```toml
# skilltap.toml — committed to the repo
[skills]
"github:acme/commit-helper" = "^1.0"
"github:acme/code-review" = "*"

[plugins]
"github:acme/dev-toolkit" = "^2.1"
```

```bash
# Teammate clones the repo, then:
skilltap sync --apply       # install exactly what skilltap.lock pins
skilltap status             # show drift between manifest, lockfile, and installed state
```

`install` and `remove` automatically update both `skilltap.toml` (the user-managed declared range) and `skilltap.lock` (the auto-managed exact resolution). Commit both files. Use the tap pattern above for catalog-style sharing; use the manifest pattern when the project needs deterministic, lockfile-pinned dependencies.

## Controlling what gets installed

For teams and organizations, skilltap lets you control the sources, not just the skills.

### Disable public registries

By default, `skilltap find` includes results from [skills.sh](https://skills.sh) — the public skills registry. For most organizations, you want to turn this off so developers only see your curated catalog:

```toml
# ~/.config/skilltap/config.toml
[registry]
enabled = []
```

With `enabled = []`, public registry results never appear in `skilltap find`. Developers only see skills from your configured taps. Include this in your team's standard config snippet.

To use your own private registry instead of the public one, see [Custom skill registry](#custom-skill-registry) below.

### Lock to org taps only

There's no allowlist setting — the simpler model is that developers only add the taps you tell them to. If they only have your company tap registered and public registries disabled, `skilltap find` only searches that tap. Nothing from the public ecosystem surfaces unless they explicitly add another tap or re-enable a registry.

The onboarding script in the next section shows this clearly: one `tap add` command, registries disabled, and that's the only source.

### Recommended org config snippet

Share this in your onboarding docs as the starting config:

```toml
[defaults]
scope = "global"
also = ["claude-code"]   # or whichever agents your team uses

[registry]
# Disable public registry search — only your tap matters
enabled = []
```

Developers paste this into `~/.config/skilltap/config.toml` and they're pointed entirely at your tap, with public registries disabled.

## Custom skill registry

For very large organizations — hundreds of skills, multiple teams contributing, or compliance requirements — a git-based tap can become unwieldy. In these cases, you can run your own **skill registry**: an HTTP service that implements a simple search API.

### When to use a custom registry

- Your catalog has hundreds or thousands of skills
- You need server-side search, filtering, or access control
- You want to track install counts or audit usage
- You need to serve skills from behind SSO or a corporate proxy

### The registry API

A custom registry implements one endpoint:

```
GET {url}/api/search?q={query}&limit={n}

Response:
{
  "skills": [
    {
      "id": "unique-identifier",
      "name": "skill-display-name",
      "description": "What this skill does",
      "source": "owner/repo",
      "installs": 1234
    }
  ]
}
```

- `source` must be a valid skilltap install ref — an `owner/repo` shorthand, a full git URL, or `npm:package`
- `installs` can be `0` if you don't track usage
- `description` can be an empty string

This is the same API that [skills.sh](https://skills.sh) uses, so any implementation compatible with one works with the other.

### Configuring a custom registry

Add it to `config.toml`:

```toml
[registry]
enabled = ["acme"]

[[registry.sources]]
name = "acme"
url = "https://skills.acme.com"
```

Now `skilltap find` searches your registry instead of (or in addition to) the public one:

```bash
$ skilltap find review

  code-reviewer      Security-focused code review    42 installs  [acme]
  pr-feedback        Inline PR feedback generator    18 installs  [acme]
```

You can enable multiple registries — results are merged in order:

```toml
[registry]
enabled = ["acme", "skills.sh"]
```

## Using third-party registries

The same mechanism works for public registries run by third parties. As the ecosystem grows, communities and tool vendors may run their own registries. Adding one is the same as adding an org registry:

```toml
[registry]
enabled = ["skills.sh", "awesome-skills"]

[[registry.sources]]
name = "awesome-skills"
url = "https://awesome-skills.example.com"
```

Results from all enabled registries appear together in `skilltap find` output, each tagged with their registry name so you know where a skill came from.

## Security scanning

Scanning is on by default (static mode) and runs locally on each developer's machine at install time. It's a useful backstop, not the primary control mechanism for most teams.

If you want to tighten it, `on_warn = "fail"` turns warnings into hard stops. Note that `on_warn` lives in the per-mode blocks (`[security.human]` for interactive use, `[security.agent]` for `--agent` / CI):

```toml
[security.human]
on_warn = "fail"

[security.agent]
on_warn = "fail"   # already the default for agent mode
```

See the [Security guide](/guide/security) for what static scanning catches and how the optional semantic scan works.

## Agent-agnostic by design

Your tap serves every agent your team uses — no per-agent catalogs, no per-agent authentication. If your developers use a mix of Claude Code, Cursor, Codex, and Gemini, they all install from the same tap.

During install, each developer optionally symlinks to their agent's directory:

```bash
# Claude Code user
skilltap install code-reviewer --also claude-code

# Cursor user
skilltap install code-reviewer --also cursor

# Both
skilltap install code-reviewer --also claude-code,cursor
```

The skill is installed once to `~/.agents/skills/` (or project-scoped to `.agents/skills/`) and symlinked wherever needed.

## Example: full onboarding script

For companies and projects, this makes a clean onboarding script:

```bash
#!/bin/bash
# onboarding.sh — run this when setting up a new dev machine

# Install skilltap
curl -fsSL https://skilltap.dev/install.sh | sh

# Add the company skill catalog
skilltap tap add acme https://gitea.acme.com/eng/acme-skills

# Install standard team skills
skilltap install code-reviewer --global --also claude-code
skilltap install pr-helper --global --also claude-code
skilltap install commit-helper --global --also claude-code

echo "Skills installed. Run 'skilltap find' to browse the full catalog."
```

For a smaller group, the same three commands in a message or README works just as well.

## Checklist

- [ ] Create a tap repo on your git host (`skilltap tap init`)
- [ ] Add skills to `tap.json`
- [ ] Share the `tap add` command with whoever should subscribe
- [ ] Let subscribers know to run `skilltap update` when skills are updated

## Related

- [Taps](/guide/taps) — full tap reference
- [Installing Skills](/guide/installing-skills) — all install options and flags
- [Security](/guide/security) — static and semantic scanning in detail
- [Configuration](/guide/configuration) — config file reference
