---
description: Share and standardize agent skills across your organization with a private tap. Set up a company catalog, onboard developers, and keep skills up to date.
---

# Using skilltap with a Team

skilltap is designed to work across teams and organizations, not just individuals. A **tap** — a git repository containing a `tap.json` index — is the primitive that makes this work. You maintain one tap, and every developer on your team installs and updates from it.

## The team model

The pattern is simple:

1. **You maintain** a tap repo on any git host your team already uses (GitHub, GitLab, Gitea, Bitbucket, self-hosted)
2. **Developers add** your tap once during onboarding
3. **Everyone installs** skills by name from the catalog
4. **When skills change**, developers run `skilltap update --all` to pull the latest

No external dependencies, no centralized service, no per-agent configuration.

## Setting up a company tap

### Create the tap

```bash
skilltap tap init acme-skills
cd acme-skills
```

This creates a directory with an empty `tap.json`. It's just a git repo — initialize it and push it to wherever your team hosts code:

```bash
git init
git add .
git commit -m "Init tap"
git remote add origin https://gitea.acme.com/eng/acme-skills
git push -u origin main
```

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

## Onboarding developers

Give every new developer one command to add the company tap:

```bash
skilltap tap add acme https://gitea.acme.com/eng/acme-skills
```

After that, they can search and install from your catalog by name — no URLs to copy-paste:

```bash
# Search the catalog
skilltap find

# Install a skill
skilltap install code-reviewer --global --also claude-code
```

Put this in your onboarding docs or team runbook alongside your other dev environment setup.

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

When the team updates a skill — bug fix, improved prompt, new capability — developers pull the latest with:

```bash
skilltap update --all
```

This fetches each installed skill, diffs only the changed lines, re-runs the scan on the diff, and applies the update. Developers see what changed before it lands.

For CI/CD or automated environments:

```bash
skilltap update --all --yes
```

## Controlling what developers can install

This is where skilltap earns its keep for organizations: you control the sources, not just the skills.

### Disable npm

By default, `skilltap find` includes npm results and `skilltap install npm:...` works. For most organizations, you want to turn this off so developers only see your curated catalog:

```toml
# ~/.config/skilltap/config.toml
[registry]
allow_npm = false
```

With `allow_npm = false`, `skilltap install npm:...` exits with an error and npm results never appear in `skilltap find`. Include this in your team's standard config snippet.

### Lock to org taps only

There's no allowlist setting — the simpler model is that developers only add the taps you tell them to. If they only have your company tap registered, `skilltap find` only searches that tap. Nothing from the public ecosystem surfaces unless they explicitly add another tap.

The onboarding script in the next section shows this clearly: one `tap add` command, and that's the only source.

### Recommended org config snippet

Share this in your onboarding docs as the starting config:

```toml
[defaults]
scope = "global"
also = ["claude-code"]   # or whichever agents your team uses

[registry]
allow_npm = false
```

Developers paste this into `~/.config/skilltap/config.toml` and they're pointed entirely at your tap, with npm disabled.

## Security scanning

Scanning is on by default (static mode) and runs locally on each developer's machine at install time. It's a useful backstop, not the primary control mechanism for most teams.

If you want to tighten it, `on_warn = "fail"` turns warnings into hard stops:

```toml
[security]
on_warn = "fail"
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

## Checklist

- [ ] Create a tap repo on your git host
- [ ] Add skills to `tap.json`
- [ ] Share the `tap add` command and recommended config snippet in your onboarding docs
- [ ] Set `allow_npm = false` in the recommended config if you want npm disabled
- [ ] Document how to run `skilltap update --all` when skills are updated

## Related

- [Taps](/guide/taps) — full tap reference, including HTTP registry taps
- [Installing Skills](/guide/installing-skills) — all install options and flags
- [Security](/guide/security) — static and semantic scanning in detail
- [Configuration](/guide/configuration) — config file reference
