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
If an HTTPS URL fails due to authentication, skilltap automatically retries with SSH (and vice versa). The URL that works is saved to `state.json` so future updates use it directly.
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

### Tap plugin

If a tap defines a `plugins` array, install a full plugin by name using `tap-name/plugin-name`:

```bash
skilltap install my-tap/dev-assistant
```

This installs all plugin components in one step: SKILL.md files, MCP server entries, and agent definition files. The plugin is recorded in `state.json::plugins[]` and tracked as a single unit.

Use `skilltap plugin` to see what's installed and manage individual components.

### Plugin auto-detection

When you install from a git URL, GitHub shorthand, or npm, skilltap checks the cloned repo for plugin metadata:

- `.claude-plugin/plugin.json`
- `.codex-plugin/plugin.json`

If found, skilltap prompts:

```
This repo contains a plugin (dev-assistant). Install as a plugin (skills + MCP + agents)?
  ● Yes — install full plugin
  ○ No — install skills only
```

Choosing "Yes" installs all components and records the plugin. Choosing "No" installs SKILL.md files only, the same as any other repo.

### MCP-only install

Prefix any source with `mcp:` to install just its MCP servers, skipping skill machinery entirely. Useful when a repo only ships `.mcp.json` (or a plugin manifest with `[[servers]]`) and you want the servers wired into your agents without scanning skills.

```bash
skilltap install mcp:user/db-tools --project
skilltap install mcp:/path/to/local/server
skilltap install mcp:npm:@scope/search-mcp
```

Servers land under `state.json::mcpServers[]` and get injected into each `--also` target's MCP config (default: `claude-code`) namespaced as `skilltap:<slug>:<server-name>`. Re-running with the same source replaces the existing entries (idempotent).

To remove, pass the same source string back:

```bash
skilltap remove mcp:user/db-tools --project
```

You cannot mix `mcp:` and regular sources in one command — run them separately.

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

::: tip Commit `.agents/state.json` and `skilltap.lock`
When you install a project-scoped skill, skilltap records it in `.agents/state.json` (and `skilltap.lock` if you've run `skilltap.toml`-based dependency management). **Commit both files** — they're the project's skill lockfile. Teammates can then see which skills the project uses, and `skilltap doctor` can verify the project's skill state is intact. v0.x users with existing `.agents/installed.json` will have it transparently migrated on the next install — `skilltap doctor --fix` cleans up the orphan `installed.json` afterward.
:::

### Smart scope default (v2.1)

If you don't pass `--global` or `--project`, skilltap **infers** the scope automatically — there is no prompt:

- Inside a git repo → `--project`
- Outside any git repo → `--global`

The inferred scope is reported in the install output. To override, pass `--project` or `--global` explicitly, or set `defaults.scope = "global"`/`"project"` in your config (`~/.config/skilltap/config.toml`).

### Recovering from a broken `skilltap.toml`

If your project's `skilltap.toml` has a TOML parse error or schema mismatch, install behavior depends on the mode:

- **Interactive mode**: install auto-recovers before doing any work. The corrupt file is backed up to `skilltap.toml.bak` (your original content is preserved there), a fresh empty manifest is written in its place, and the install proceeds. You'll see a warning in the install output explaining what happened.

- **Agent mode** (`--agent` or `SKILLTAP_AGENT=1`): install refuses and exits 1 with a pointer to `skilltap doctor --fix`. The corrupt file is left untouched — scripts and CI shouldn't silently mutate your committed files. Run `skilltap doctor --fix` to perform the same backup-and-reset, then retry the install.

You can also run `skilltap doctor --fix` directly any time to recover a corrupt `skilltap.toml` (or `skilltap.lock`) without performing an install.

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

## Managing plugins

After installing a plugin, use `skilltap plugin` to see all installed plugins:

```bash
skilltap plugin
```

To see the components of a specific plugin:

```bash
skilltap plugin info dev-assistant
```

To toggle specific components on or off:

```bash
skilltap plugin toggle dev-assistant
```

To remove a plugin and all its components:

```bash
skilltap plugin remove dev-assistant
```

See the [CLI reference](/reference/cli#skilltap-plugin) for all plugin commands and flags.
