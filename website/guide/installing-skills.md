---
description: Install agent skills from GitHub, GitLab, Gitea, SSH, npm, or local paths. Covers scopes, flags, agent symlinks, multi-skill repos, and tap name resolution.
---

# Installing Skills

skilltap can install skills from git URLs, GitHub shorthand, SSH, tap names, and local paths. This guide covers all the ways to install, the flags available, and how scope and agent symlinks work.

## Source formats

### Git URL (any host)

Install from any git-accessible URL:

```bash
skilltap install https://gitea.example.com/user/commit-helper
skilltap install https://gitlab.com/team/code-review
skilltap install https://github.com/user/my-skill
```

This works with any host that supports `git clone` -- GitHub, GitLab, Gitea, Bitbucket, your company's private server. The repo just needs a `SKILL.md` somewhere in the tree. No `tap.json` or special structure required -- skilltap clones the repo and scans for `SKILL.md` files automatically.

::: tip Want discoverability?
Any repo with a `SKILL.md` is installable by URL, but adding your skills to a [tap](/guide/taps) gives them names, descriptions, and tags so others can find them with `skilltap find`.
:::

### SSH

```bash
skilltap install git@github.com:user/commit-helper.git
skilltap install ssh://git@gitlab.com/team/code-review.git
```

Uses your existing SSH keys. No extra configuration needed.

::: tip Automatic protocol fallback
If an HTTPS URL fails due to authentication, skilltap automatically retries with SSH (and vice versa). The URL that works is saved to `installed.json` so future updates use it directly.
:::

### GitHub shorthand

If the source contains a `/` and no protocol, skilltap treats it as a GitHub repo:

```bash
skilltap install user/commit-helper
```

This is equivalent to `https://github.com/user/commit-helper`. You can also be explicit:

```bash
skilltap install github:user/commit-helper
```

### Tap name

If you have taps configured, install by skill name:

```bash
skilltap install commit-helper
```

skilltap searches all your configured taps for a skill with that name, resolves the repo URL, and installs it.

### Tap name with version

Pin to a specific branch or tag:

```bash
skilltap install commit-helper@v1.2.0
skilltap install commit-helper@main
```

### Local path

Install from a directory on your filesystem:

```bash
skilltap install ./my-skill
skilltap install /home/nathan/dev/my-skill
skilltap install ~/skills/my-skill
```

The directory must contain a `SKILL.md` file.

### npm package

Install a skill published to the npm registry:

```bash
skilltap install npm:vibe-rules
skilltap install npm:@scope/my-skills
skilltap install npm:vibe-rules@1.0.0
```

The `npm:` prefix downloads the tarball from the npm registry and verifies its SHA-512 integrity before installing. This gives you access to any skill published as an npm package.

**Version pinning:** Append `@version` or a dist-tag (e.g. `@latest`). Without a version, the `latest` dist-tag is used.

**Private registries:** skilltap reads your `.npmrc` file or `NPM_CONFIG_REGISTRY` environment variable automatically.

**Updates:** npm-sourced skills update by comparing version numbers rather than git SHAs. `skilltap update` fetches the latest version from the registry and replaces the skill if the version differs.

### Multiple sources

Install several skills in a single command by listing them as positional arguments:

```bash
skilltap install skill-a skill-b skill-c --global
```

Each source is installed in sequence using the same scope, flags, and security policy. Errors are collected and shown together at the end — a failure on one source does not prevent the others from being attempted.

## Scope: global vs project

Every skill is installed to either a **global** or **project** scope.

### Global scope

```bash
skilltap install commit-helper --global
```

Installs to `~/.agents/skills/commit-helper/`. Available everywhere on your machine.

### Project scope

```bash
skilltap install commit-helper --project
```

Installs to `.agents/skills/commit-helper/` inside the current project (determined by the nearest `.git` directory). Only available when working in that project.

::: tip Commit `.agents/installed.json`
When you install a project-scoped skill, skilltap records it in `.agents/installed.json` at the project root. **Commit this file** — it's the project's skill lockfile. Teammates can then see which skills the project uses, and `skilltap doctor` can verify the project's skill state is intact.
:::

### Prompted scope

If you don't pass `--global` or `--project`, skilltap asks:

```
Install to:
  ● Global (~/.agents/skills/)
  ○ Project (.agents/skills/)
```

You can set a default scope in your config so you're never prompted. See [Configuration](/guide/configuration).

::: tip
The `--yes` flag does **not** skip the scope prompt. Use `--yes --global` or `--yes --project` for fully non-interactive installs.
:::

## Agent symlinks

During interactive install, skilltap prompts you to choose which agents the skill should be visible to:

```
◆ Which agents should this skill be available to? (space to toggle, enter to confirm)
│ ◼ Claude Code
│ ◻ Cursor
│ ◻ Codex
│ ◻ Gemini
│ ◻ Windsurf
```

If your selection differs from your saved default, you'll be asked whether to save it:

```
◆ Save agent selection as default?
│ No
```

You can select none — the skill will only be installed to `.agents/skills/`.

The agent prompt is **skipped** when:
- `--also` is passed explicitly (e.g. `--also claude-code`)
- `--yes` is set
- `config.defaults.also` is non-empty (you've saved a default via `skilltap config` or a previous install)

You can also specify agents directly via `--also`:

```bash
skilltap install commit-helper --global --also claude-code --also cursor
```

Supported agent identifiers:

| Identifier | Global symlink path | Project symlink path |
|---|---|---|
| `claude-code` | `~/.claude/skills/` | `.claude/skills/` |
| `cursor` | `~/.cursor/skills/` | `.cursor/skills/` |
| `codex` | `~/.codex/skills/` | `.codex/skills/` |
| `gemini` | `~/.gemini/skills/` | `.gemini/skills/` |
| `windsurf` | `~/.windsurf/skills/` | `.windsurf/skills/` |

### Default agent symlinks

You can set a default agent selection in your config, either via the "Save as default?" prompt during install or manually:

```toml
# ~/.config/skilltap/config.toml
[defaults]
also = ["claude-code", "cursor"]
```

When defaults are set, the agent selection prompt is skipped entirely — the saved selection is used automatically. To change agents for a single install, pass `--also` explicitly. To change the default permanently, re-run `skilltap config`.

## Multi-skill repos

Some repos contain multiple skills -- for example, a project with both a development workflow skill and a review checklist skill. There's no special manifest needed; skilltap discovers all `SKILL.md` files in the repo automatically.

When skilltap finds multiple `SKILL.md` files, it prompts you to choose:

```
$ skilltap install https://gitea.example.com/user/termtube

Found 2 skills in user/termtube:
  [1] termtube-dev        Development workflow for termtube
  [2] termtube-review     Code review checklist for termtube

Install which? (1,2,all): 1
```

With `--yes`, all skills are auto-selected:

```
$ skilltap install https://gitea.example.com/user/termtube --yes --global

Found 2 skills: termtube-dev, termtube-review
Auto-selecting all (--yes)

✓ Installed termtube-dev → ~/.agents/skills/termtube-dev/
✓ Installed termtube-review → ~/.agents/skills/termtube-review/
```

## Security during install

Every install runs a static security scan. For a clean skill (no warnings), you're asked to confirm before anything is written to disk:

```
◇  Static scan: 0 warnings
│
◇  Install code-reviewer?
│  › Yes
```

With `--yes`, this final confirmation is skipped and the skill is installed automatically.

If warnings are found, you'll see them and be asked how to proceed:

```
⚠ Static warnings in some-skill:
  SKILL.md L14: Invisible Unicode (3 zero-width chars)

? Run semantic scan?
  ● Yes
  ○ No
```

If you choose to run the semantic scan and haven't set up an agent yet, skilltap detects available agent CLIs on your machine and lets you pick one. Your choice is saved for future installs.

After all scans complete, if there are still warnings you're asked to confirm:

```
? Install some-skill despite warnings?
  ○ Yes
  ● No
```

With `--strict`, any warning aborts immediately — no prompt. With `--skip-scan`, scanning is bypassed entirely (blocked if `require_scan = true` in config).

For the full security model, see [Security](./security).

## Flags reference

| Flag | Description |
|---|---|
| `--global` | Install to `~/.agents/skills/` |
| `--project` | Install to `.agents/skills/` in the current project |
| `--ref <ref>` | Install a specific branch or tag (git sources only) |
| `--also <agent>` | Also symlink to an agent's directory. Repeatable. Skips the agent selection prompt. |
| `--yes` | Auto-select all skills, auto-accept clean installs, skip agent selection prompt |
| `--strict` | Abort if any security warnings are found (exit 1) |
| `--no-strict` | Override `on_warn = "fail"` in config for this invocation |
| `--semantic` | Run Layer 2 semantic scan (auto-runs — no prompt shown) |
| `--skip-scan` | Skip security scanning entirely (not recommended) |

### Fully non-interactive install

To install without any prompts (for scripts and CI):

```bash
skilltap install user/commit-helper --global --yes --strict
```

This auto-selects all skills, uses global scope, and aborts if any security issue is found.

## Reinstalling / updating

If you try to install a skill that's already installed, skilltap detects the conflict right after skill selection and asks:

```
◆ commit-helper is already installed. Update it instead?
│ Yes
```

Choosing yes runs `skilltap update` for that skill. With `--yes` or in agent mode, the update happens automatically without prompting.

To force a clean reinstall, remove the skill first:

```bash
skilltap remove commit-helper && skilltap install user/commit-helper --global
```

## Removing skills

```bash
skilltap remove commit-helper
```

This removes the skill directory and any agent symlinks. You'll be asked to confirm unless you pass `--yes`.

Remove multiple skills in one command:

```bash
skilltap remove skill-a skill-b skill-c --yes
```

Or omit the name entirely to pick interactively from all installed skills:

```bash
skilltap remove
```

Use `--project` or `--global` to target a specific scope:

```bash
skilltap remove termtube-dev --project
skilltap remove commit-helper --global
```

## Linking local skills

For development, you can symlink a local skill directory into the install path instead of cloning:

```bash
skilltap link ./my-skill --also claude-code
```

```
✓ Linked my-skill → ~/.agents/skills/my-skill/
✓ Symlinked → ~/.claude/skills/my-skill/
```

This creates a symlink, not a copy. Changes to your local directory are immediately visible to the agent. Useful when developing and testing a skill.

Link to project scope:

```bash
skilltap link ./my-skill --project --also claude-code
```

Remove the link (does not delete your original directory):

```bash
skilltap unlink my-skill
```

Linked skills are skipped during `skilltap update` since they're managed directly by you.
