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

This works with any host that supports `git clone` -- GitHub, GitLab, Gitea, Bitbucket, your company's private server.

### SSH

```bash
skilltap install git@github.com:user/commit-helper.git
skilltap install ssh://git@gitlab.com/team/code-review.git
```

Uses your existing SSH keys. No extra configuration needed.

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

By default, skills are installed to `.agents/skills/` only. To also make a skill visible to a specific agent, use `--also`:

```bash
skilltap install commit-helper --global --also claude-code
```

This creates:
- `~/.agents/skills/commit-helper/` (the actual files)
- `~/.claude/skills/commit-helper/` (a symlink to the above)

You can specify multiple agents:

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

If you always want to symlink to the same agents, set it in your config:

```toml
# ~/.config/skilltap/config.toml
[defaults]
also = ["claude-code", "cursor"]
```

Now every `skilltap install` automatically symlinks to both agents without needing `--also`.

## Multi-skill repos

Some repos contain multiple skills -- for example, a project with both a development workflow skill and a review checklist skill.

When skilltap finds multiple `SKILL.md` files in a repo, it prompts you to choose:

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

## Flags reference

| Flag | Description |
|---|---|
| `--global` | Install to `~/.agents/skills/` |
| `--project` | Install to `.agents/skills/` in the current project |
| `--also <agent>` | Also symlink to an agent's directory. Repeatable. |
| `--ref <ref>` | Install a specific branch or tag |
| `--yes` | Auto-select all skills and auto-accept clean installs |
| `--strict` | Abort if any security warnings are found (exit 1) |
| `--no-strict` | Override `on_warn = "fail"` in config for this invocation |
| `--semantic` | Run Layer 2 semantic security scan |
| `--skip-scan` | Skip security scanning entirely (not recommended) |

### Fully non-interactive install

To install without any prompts (for scripts and CI):

```bash
skilltap install user/commit-helper --global --yes --strict
```

This auto-selects all skills, uses global scope, and aborts if any security issue is found.

## Removing skills

```bash
skilltap remove commit-helper
```

```
Remove commit-helper (global, v1.2.0)? (y/N): y
✓ Removed commit-helper
```

This removes the skill directory and any agent symlinks. Use `--project` to remove from project scope:

```bash
skilltap remove termtube-dev --project
```

Skip the confirmation prompt:

```bash
skilltap remove commit-helper --yes
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
