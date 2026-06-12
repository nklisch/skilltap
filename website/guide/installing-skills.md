---
description: Install agent skills from GitHub, GitLab, Gitea, SSH, npm, or local paths. Covers scopes, flags, agent symlinks, multi-skill repos, and tap name resolution.
---

# Installing Skills

skilltap can install skills from git URLs, GitHub shorthand, SSH, tap names, and local paths. This guide covers all the ways to install, the flags available, and how scope and agent symlinks work.

## Source formats

### Git URL (any host)

Install from any git-accessible URL:

```bash
skilltap install skill https://gitea.example.com/user/commit-helper
skilltap install skill https://gitlab.com/team/code-review
skilltap install skill https://github.com/user/my-skill
```

This works with any host that supports `git clone` -- GitHub, GitLab, Gitea, Bitbucket, your company's private server. The repo just needs a `SKILL.md` somewhere in the tree. No `tap.json` or special structure required -- skilltap clones the repo and scans for `SKILL.md` files automatically.

::: tip Want discoverability?
Any repo with a `SKILL.md` is installable by URL, but adding your skills to a [tap](/guide/taps) gives them names, descriptions, and tags so others can find them with `skilltap find`.
:::

### SSH

```bash
skilltap install skill git@github.com:user/commit-helper.git
skilltap install skill ssh://git@gitlab.com/team/code-review.git
```

Uses your existing SSH keys. No extra configuration needed.

::: tip Automatic protocol fallback
If an HTTPS URL fails due to authentication, skilltap automatically retries with SSH (and vice versa). The URL that works is saved to `state.json` so future updates use it directly.
:::

### GitHub shorthand

If the source contains a `/` and no protocol, skilltap treats it as a GitHub repo:

```bash
skilltap install skill user/commit-helper
```

This is equivalent to `https://github.com/user/commit-helper`. You can also be explicit:

```bash
skilltap install skill github:user/commit-helper
```

This also works for repos that package skills under `skills/` or `.agents/skills/`.
For example, TweetClaw ships an X/Twitter automation skill in `skills/tweetclaw/`:

```bash
skilltap install skill Xquik-dev/tweetclaw --scope global --also codex --yes
```

With `--yes`, skilltap auto-selects discovered skills from the repo and keeps
the command usable in CI, setup scripts, and agent bootstrapping flows.

### Tap name

If you have taps configured, install by skill name:

```bash
skilltap install skill commit-helper
```

skilltap searches all your configured taps for a skill with that name, resolves the repo URL, and installs it.

### Tap name with version

Pin to a specific branch or tag:

```bash
skilltap install skill commit-helper@v1.2.0
skilltap install skill commit-helper@main
```

### Local path

Install from a directory on your filesystem:

```bash
skilltap install skill ./my-skill
skilltap install skill /home/nathan/dev/my-skill
skilltap install skill ~/skills/my-skill
```

The directory must contain a `SKILL.md` file.

### npm package

Install a skill published to the npm registry:

```bash
skilltap install skill npm:vibe-rules
skilltap install skill npm:@scope/my-skills
skilltap install skill npm:vibe-rules@1.0.0
```

The `npm:` prefix downloads the tarball from the npm registry and verifies its SHA-512 integrity before installing. This gives you access to any skill published as an npm package.

**Version pinning:** Append `@version` or a dist-tag (e.g. `@latest`). Without a version, the `latest` dist-tag is used.

**Private registries:** skilltap reads your `.npmrc` file or `NPM_CONFIG_REGISTRY` environment variable automatically.

**Updates:** npm-sourced skills update by comparing version numbers rather than git SHAs. `skilltap update` fetches the latest version from the registry and replaces the skill if the version differs.

### Tap plugin

If a tap defines a `plugins` array, install a full plugin by name using `tap-name/plugin-name`:

```bash
skilltap install plugin my-tap/dev-assistant
```

This installs all plugin components in one step: SKILL.md files, MCP server entries, and agent definition files. The plugin is recorded in `state.json::plugins[]` and tracked as a single unit.

Use `skilltap status`, `skilltap info <name>`, and `skilltap toggle plugin <name>[:<component>]` to manage installed plugins and their individual components.

### Multi-plugin repos

A single repo can ship multiple plugins. Pick which one to install with `:plugin-name`, or install them all with `:*`:

```bash
# Install one plugin out of a repo that ships several
skilltap install plugin acme/tools:auth

# Install every plugin defined in the repo (one record per plugin)
skilltap install plugin acme/tools:*
```

When you pin a ref with `@`, put the selector before it: `acme/tools:auth@v1.2.0`.

### Plugin Capture

If a plugin name already exists in your state from a different source — for example, you adopted a Claude Code plugin and now want to manage the same name from skilltap — `install plugin` prompts to **capture** the existing entry into the new source rather than installing side-by-side. The capture flow rewrites the source and refreshes the install in place, so symlinks and components stay continuous.

Two flags control capture in non-interactive contexts (mutually exclusive — passing both is an error):

| Flag | Effect |
|---|---|
| `--force-capture` | Capture without prompting. Use when you know the existing entry should be replaced. |
| `--no-capture` | Skip capture even on a name match; install side-by-side under a derived name. |

```bash
# Take ownership of an existing plugin record from a new source
skilltap install plugin acme/dev-toolkit --force-capture

# Refuse to overwrite — install both side-by-side
skilltap install plugin acme/dev-toolkit --no-capture
```

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

Use the `mcp` install type to install just MCP servers, skipping skill machinery entirely. Useful when a repo only ships `.mcp.json` (or a plugin manifest with `[[servers]]`) and you want the servers wired into your agents without scanning skills.

```bash
skilltap install mcp user/db-tools --scope project
skilltap install mcp /path/to/local/server
skilltap install mcp npm:@scope/search-mcp
```

Servers land under `state.json::mcpServers[]` and get injected into each `--also` target's MCP config (default: `claude-code`) namespaced as `skilltap:<slug>:<server-name>`. Re-running with the same source replaces the existing entries (idempotent).

`install mcp` honors the smart-scope default like `install skill` and `install plugin`: outside a git repo, an MCP installs to global scope without prompting; inside a git repo, it defaults to project scope.

To remove, use `remove mcp` with the server name:

```bash
skilltap remove mcp db-tools --scope project
```

Each install command takes one type — to install a skill and an MCP from the same repo, run `install skill` and `install mcp` separately (or use a plugin install if the repo defines one).

## Scope: global vs project

Every skill is installed to either a **global** or **project** scope. The canonical scope flag is `--scope project|global`.

### Global scope

```bash
skilltap install skill commit-helper --scope global
```

Installs to `~/.agents/skills/commit-helper/`. Available everywhere on your machine.

### Project scope

```bash
skilltap install skill commit-helper --scope project
```

Installs to `.agents/skills/commit-helper/` inside the current project (determined by the nearest `.git` directory). Only available when working in that project.

::: tip Commit `.agents/state.json` and `skilltap.lock`
When you install a project-scoped skill, skilltap records it in `.agents/state.json` (and `skilltap.lock` if you've run `skilltap.toml`-based dependency management). **Commit both files** — they're the project's skill lockfile. Teammates can then see which skills the project uses, and `skilltap doctor` can verify the project's skill state is intact.
:::

### Smart scope default

If you don't pass `--scope`, skilltap **infers** the scope automatically — there is no prompt:

- Inside a git repo → `project`
- Outside any git repo → `global`

The inferred scope is reported in the install output, e.g. `→ scope: project (inferred from cwd)`. To override, pass `--scope project` / `--scope global` explicitly, or set `defaults.scope = "global"`/`"project"` in your config (`~/.config/skilltap/config.toml`).

### Recovering from a broken `skilltap.toml`

If your project's `skilltap.toml` has a TOML parse error or schema mismatch, install behavior depends on the mode:

- **Interactive mode** (TTY attached): install auto-recovers before doing any work. The corrupt file is backed up to `skilltap.toml.bak` (your original content is preserved there), a fresh empty manifest is written in its place, and the install proceeds. You'll see a warning in the install output explaining what happened.

- **Non-interactive automation** (`--yes`, piped stdout, or any non-TTY context): install refuses and exits 1 with a pointer to `skilltap doctor --fix`. The corrupt file is left untouched — scripts and CI shouldn't silently mutate your committed files. Run `skilltap doctor --fix` to perform the same backup-and-reset, then retry the install.

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

You can also specify agents directly via `--also`. `--also` is repeatable — pass it once per agent:

```bash
skilltap install skill commit-helper --scope global --also claude-code --also cursor
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
$ skilltap install skill https://gitea.example.com/user/termtube

Found 2 skills in user/termtube:
  [1] termtube-dev        Development workflow for termtube
  [2] termtube-review     Code review checklist for termtube

Install which? (1,2,all): 1
```

With `--yes`, all skills are auto-selected:

```
$ skilltap install skill https://gitea.example.com/user/termtube --yes --scope global

Found 2 skills: termtube-dev, termtube-review
Auto-selecting all (--yes)

✓ Installed termtube-dev → ~/.agents/skills/termtube-dev/
✓ Installed termtube-review → ~/.agents/skills/termtube-review/
```

### Multiple sources

Install several skills in a single command by listing them as positional arguments after the type:

```bash
skilltap install skill skill-a skill-b skill-c --scope global
```

Each source is installed in sequence using the same scope, flags, and security policy. Errors are collected and shown together at the end — a failure on one source does not prevent the others from being attempted.

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

With `--strict`, any warning aborts immediately — no prompt. With `--skip-scan`, scanning is bypassed entirely (matching the behavior when the source URL hits a `security.trust` glob).

For the full security model, see [Security](./security).

## Flags reference

| Flag | Description |
|---|---|
| `--scope project\|global` | Install scope. Defaults to smart-scope (project inside a git repo, global otherwise). |
| `--ref <ref>` | Install a specific branch or tag (git sources only) |
| `--also <agent>` | Also symlink to an agent's directory. Repeatable (`--also a --also b`). Skips the agent selection prompt. |
| `--yes` | Auto-select all skills, auto-accept clean installs, skip agent selection prompt |
| `--json` | Emit machine-readable output |
| `--strict` | Abort if any security warnings are found (exit 1) |
| `--semantic` | Run Layer 2 semantic scan (auto-runs — no prompt shown) |
| `--skip-scan` | Skip security scanning entirely (not recommended) |
| `--force-capture` | (`install plugin`) Capture an existing same-named plugin into the new source without prompting |
| `--no-capture` | (`install plugin`) Refuse capture; install side-by-side instead |

### Fully non-interactive install

To install without any prompts (for scripts and CI):

```bash
skilltap install skill user/commit-helper --scope global --yes --strict
```

This auto-selects all skills, uses global scope, and aborts if any security issue is found.

## Reinstalling / updating

If you try to install a skill that's already installed, skilltap detects the conflict right after skill selection and asks:

```
◆ commit-helper is already installed. Update it instead?
│ Yes
```

Choosing yes runs `skilltap update` for that skill. With `--yes` (or any non-interactive context), the update happens automatically without prompting.

To force a clean reinstall, remove the skill first:

```bash
skilltap remove skill commit-helper && skilltap install skill user/commit-helper --scope global
```

## Removing skills

```bash
skilltap remove skill commit-helper
```

This removes the skill directory and any agent symlinks. You'll be asked to confirm unless you pass `--yes`.

Remove multiple skills in one command:

```bash
skilltap remove skill skill-a skill-b skill-c --yes
```

Or omit the name entirely to pick interactively from all installed skills:

```bash
skilltap remove skill
```

Use `--scope` to target a specific scope:

```bash
skilltap remove skill termtube-dev --scope project
skilltap remove skill commit-helper --scope global
```

## Adopting local skills

For development — or to bring an existing skill directory under skilltap management — use `skilltap adopt` to track a local directory in place (no copy, no symlink dance required):

```bash
skilltap adopt ./my-skill --also claude-code
```

```
✓ Adopted my-skill (tracking in place: ./my-skill)
✓ Symlinked → ~/.claude/skills/my-skill/
```

`adopt` records the skill in `state.json` pointing at its current location. Changes to your local directory are immediately visible to the agent. Useful when developing and testing a skill.

Adopt into project scope:

```bash
skilltap adopt ./my-skill --scope project --also claude-code
```

To physically move the directory under the canonical install path while adopting, pass `--move`:

```bash
skilltap adopt ./my-skill --move
```

Stop tracking an adopted skill:

```bash
skilltap remove skill my-skill
```

Adopted skills are skipped during `skilltap update` since they're managed directly by you.

## Managing plugins

After installing a plugin, use `skilltap status` for the unified dashboard, or look up a single plugin by name:

```bash
skilltap status                   # unified view: skills, plugins, MCPs
skilltap info dev-assistant       # show one item's components and source
```

To toggle a plugin or one of its components on or off:

```bash
skilltap toggle plugin dev-assistant
skilltap toggle plugin dev-assistant:some-mcp   # toggle a single component
```

To remove a plugin and all its components:

```bash
skilltap remove plugin dev-assistant
```

See the [CLI reference](/reference/cli) for all flags.
