# skilltap

**Homebrew taps for agent skills.** Install, manage, and share AI agent skills from any git host — agent-agnostic, multi-source, secure.

**[skilltap.dev](https://skilltap.dev)** — docs, guides, and skill discovery.

```bash
curl -fsSL https://skilltap.dev/install.sh | sh
```

## Why skilltap?

The [SKILL.md format](https://agentskills.io/specification) is supported by 40+ agents — Claude Code, Cursor, Codex CLI, Gemini CLI, and more. Writing skills is easy. Distributing them is not.

skilltap fills that gap:

- **Host your own tap.** A tap is just a git repo with a JSON index — like a Homebrew formula tap. Stand one up in minutes for your team, your friends, or yourself. No registry account, no upload portal.
- **Any git host.** Install from GitHub, GitLab, Gitea, Bitbucket, or a private server. Your SSH keys and credential helpers just work.
- **Agent-agnostic.** One install lands in `~/.agents/skills/`. Opt in to symlinking to Claude Code, Cursor, Codex, Gemini, or Windsurf with `--also`.
- **Source-tracked updates.** skilltap remembers where every skill came from. `skilltap update` fetches upstream changes, shows a diff, re-scans changed lines, and asks before applying.
- **Security scanning.** Every install runs a static scan (invisible Unicode, hidden HTML, obfuscated code, suspicious URLs, tag injection) before anything lands on disk.
- **Standalone binary.** One file, no runtime dependencies.

## Install

**curl (recommended):**

```bash
curl -fsSL https://skilltap.dev/install.sh | sh
```

Installs to `~/.local/bin/skilltap`. Override the install directory:

```bash
curl -fsSL https://skilltap.dev/install.sh | SKILLTAP_INSTALL=/usr/local/bin sh
```

**Homebrew:**

```bash
brew install nklisch/skilltap/skilltap
```

**Without installing:**

```bash
bunx skilltap --help   # requires Bun
npx skilltap --help    # requires Bun on PATH
```

Or download a binary directly from [GitHub Releases](https://github.com/nklisch/skilltap/releases).

## Quickstart

```bash
# See what's installed (skills, plugins, taps, drift) — opens TUI dashboard in TTY
skilltap

# Browse skills from the built-in community tap
skilltap find

# Install a skill (type is required: skill | plugin | mcp)
# Smart scope: project if inside a git repo, global otherwise
skilltap install skill commit-helper --also claude-code

# Install from any git URL
skilltap install skill https://github.com/you/my-skill --scope global

# Install a plugin (skills + MCP servers + agent definitions in one)
skilltap install plugin corp/dev-toolkit --also claude-code

# Preview a source without installing
skilltap try skill someone/their-skill

# Headless status dashboard
skilltap status

# Update everything
skilltap update
```

## Project manifests

Declare your project's skill + plugin dependencies in `skilltap.toml`, commit it,
and have teammates run `skilltap sync` to bring their machines to parity. Like
`Cargo.toml` for AI agent skills.

```toml
# skilltap.toml — at your project root
[targets]
also  = ["claude-code", "cursor"]
scope = "project"

[skills]
"github:nathan/commit-helper" = "*"
"npm:@corp/code-review"       = "*"

[plugins]
"github:corp/dev-toolkit"     = "*"

[taps]
home = "https://gitea.example.com/nathan/my-tap"
```

When `skilltap.toml` is present, `skilltap install` and `skilltap remove` keep
the manifest and `skilltap.lock` in sync automatically. Run `skilltap sync`
to see drift and `skilltap sync --apply` to bring installed state in line.

```bash
skilltap install skill commit-helper   # adds to skilltap.toml + skilltap.lock
skilltap remove skill commit-helper    # drops from manifest + lockfile
skilltap sync                          # show drift between manifest, lockfile, state
skilltap sync --apply                  # execute the plan
skilltap status                        # rich snapshot: skills, plugins, MCPs, drift
```

To publish a repo as a plugin (skills + MCP servers + agent definitions),
add `.skilltap/<plugin-name>.toml` with `publish = true`. See
[the spec](docs/SPEC.md#project-manifest-and-lockfile) for the full
manifest format.

If you're upgrading from any pre-v2.2 release, run `skilltap migrate` once to
translate the config schema and consolidate state files. `loadConfig` hard-fails
on legacy shapes and points you here — there is no silent fallback.

## Taps

A **tap** is a git repo containing a `tap.json` index of skills. Taps make discovery and curation easy — and anyone can create one.

```bash
# Create your own tap (a git repo + tap.json index)
skilltap tap init my-skills
# push to any git host, then share the URL

# Subscribe to any tap
skilltap tap add acme https://gitea.acme.com/eng/acme-skills

# Search across all your taps
skilltap find review

# Install by name from a tap
skilltap install skill code-reviewer

# Update all taps
skilltap tap update
```

The built-in `skilltap-skills` tap is always available — no setup required.

## Host Your Own Tap

Whether you're managing skills for a company, a group of friends, or just yourself, a tap is all you need. It's a git repo — host it anywhere you already have git.

```bash
# Create the tap once (engineering lead, project owner, whoever)
skilltap tap init acme-skills
# add skills to tap.json, push to your git host

# Everyone else adds it once
skilltap tap add acme https://gitea.acme.com/eng/acme-skills

# Install and update by name from then on — no URLs to copy-paste
skilltap install skill code-reviewer --scope global --also claude-code
skilltap update
```

When you update a skill in the tap, every subscriber sees the diff and confirms before it applies. Your existing SSH keys and credential helpers handle authentication. See [Host Your Own Tap](https://skilltap.dev/guide/teams) for the full setup guide and config options.

## Commands

| Command | Description |
|---|---|
| `(no args)` | TUI dashboard — skills, plugins, MCPs, taps, drift (TTY only) |
| `status [--json]` | Headless dashboard — same content, safe to pipe |
| `install skill <source>` | Install a skill from a URL, GitHub shorthand, npm package, or tap name |
| `install plugin <source>` | Install a plugin (skills + MCP servers + agent definitions) |
| `install mcp <source>` | Install a standalone MCP server |
| `remove skill <name>` | Remove an installed skill |
| `remove plugin <name>` | Remove a plugin and all its components |
| `remove mcp <name>` | Remove a standalone MCP server |
| `update [type] [name]` | Update one, all of a type, or everything |
| `toggle [type] [name[:component]]` | Toggle active state; bare opens TUI picker |
| `info <name>` | Show details about an installed skill or plugin |
| `find [query]` | Search skills across configured taps (TUI when interactive) |
| `try <type> <source>` | Preview a source without installing |
| `adopt [path]` | Bring an external skill or agent plugin into skilltap (replaces `link`) |
| `move <name>` | Move a skill between global and project scope |
| `sync` | Show drift between `skilltap.toml`, `skilltap.lock`, and installed state |
| `sync --apply` | Execute the sync plan via install/remove |
| `migrate` | One-shot upgrade from any prior config/state format |
| `doctor [skill\|plugin <path>]` | Environment + state check; per-artifact validation (replaces `verify`) |
| `create [name]` | Scaffold a new skill from a template |
| `completions <shell>` | Generate shell tab-completion script |
| `tap add <name> <url>` | Add a git tap |
| `tap remove <name>` | Remove a tap |
| `tap update [name]` | Update one or all taps |
| `tap list` | List configured taps |
| `tap init <name>` | Initialize a new tap directory |
| `config get\|set\|edit\|security` | Read/write config values; interactive security wizard |
| `self-update` | Update the skilltap binary in-place |

Most commands accept `--scope project|global` for scope and `--yes` to skip prompts. **Smart scope default**: inside a git repo, `install` defaults to project scope; outside, global. There is no `--agent` flag — use `--yes` and piped stdin for non-interactive use.

## How it works

Skills are directories containing a `SKILL.md` file. skilltap installs them to `~/.agents/skills/<name>/` (global) or `.agents/skills/<name>/` (project), then creates symlinks at each agent's expected location (`.claude/skills/`, `.cursor/skills/`, etc.) so every agent picks them up automatically.

## Security

Every install and update runs a two-layer security scan before anything lands on disk.

**Static scan** (always on by default): checks for invisible Unicode, hidden HTML/CSS, markdown injection, obfuscated code, suspicious URLs, dangerous shell patterns, and tag injection.

**Semantic scan** (optional, `--semantic`): sends skill content to your local AI agent in bounded 2000-char chunks. The agent is invoked with tools disabled (`--no-tools`), so even a skill that tricks the reviewer can't cause it to take actions. Content is wrapped in a randomly-suffixed untrusted block so the agent can't be hijacked by the skill it's reviewing. Closing tags that could escape the wrapper are detected and escaped before the chunk is sent. Up to 4 chunks are evaluated in parallel; agent failures are fail-open (scan continues).

```bash
skilltap install skill my-skill --semantic   # enable semantic scan
skilltap install skill my-skill --strict     # block on any warning
skilltap install skill my-skill --skip-scan  # bypass scanning (trusted sources)
```

See [docs/SECURITY.md](docs/SECURITY.md) for the full threat model, detector reference, and configuration options.

## Non-interactive use (AI agents, CI, scripts)

skilltap uses TTY detection for output mode — piped output is already plain text. Combine with `--yes` for fully non-interactive installs:

```bash
# Fully non-interactive (piped stdout = plain output, --yes = no prompts)
skilltap install skill commit-helper --scope global --yes --skip-scan | cat

# JSON output for scripting
skilltap status --json
skilltap install skill commit-helper --yes --json
```

There is no `--agent` flag. TTY detection + `--yes` + `--json` covers all automation use cases.

## Configuration

Config is stored at `~/.config/skilltap/config.toml`. Run the interactive wizard:

```bash
skilltap config
```

Key settings: default scope (`global`/`project`), additional agent symlinks (`--also`), security scan mode (`semantic`/`static`/`none`), and `on_warn` behavior (`prompt`/`fail`/`install`). Trusted sources (your own org's repos, internal taps) skip scanning via the `security.trust = [...]` glob array.

## Authoring Skills

```bash
# Scaffold a new skill interactively
skilltap create my-skill

# Edit SKILL.md, then test locally (adopt replaces link/unlink)
skilltap adopt ./my-skill --also claude-code

# Validate before sharing (doctor replaces verify)
skilltap doctor skill my-skill/

# Push to git and share
git push -u origin main
```

Others can install with: `skilltap install skill you/my-skill`

To publish to npm (with provenance), use `--template npm`. The generated GitHub Actions workflow handles publishing automatically on release.

## Trust Signals

Skills from npm show provenance status when installed:

```
$ skilltap status

Global (.agents/skills/) — 2 skills
  Name           Status   Agents       Source
  my-npm-skill   managed  claude-code  npm:@user/my-npm-skill
  git-skill      managed  claude-code  home
```

Trust tiers: `provenance` (Sigstore/SLSA verified), `publisher` (npm identity verified), `curated` (tap-verified), `unverified`.

## Shell Completions

```bash
skilltap completions bash --install
skilltap completions zsh --install    # then: fpath=(~/.zfunc $fpath) && autoload -Uz compinit && compinit
skilltap completions fish --install
```

## Troubleshooting

```bash
skilltap doctor        # check environment, config, and installed state
skilltap doctor --fix  # auto-repair common issues (broken symlinks, orphan records)
skilltap doctor --json # machine-readable output for CI
```

## Gotchas

- **`install` requires a type subcommand.** `skilltap install commit-helper` is an error. Use `skilltap install skill commit-helper`.
- **`--yes` does not bypass security warnings.** Use `--strict` to turn warnings into hard failures, or `--skip-scan` to bypass entirely. Add trusted sources to `security.trust = [...]` in config to skip scanning for them automatically.
- **Agent symlinks are not automatic.** Pass `--also <agent>` or set defaults in config. The skill always lands in `.agents/skills/` — symlinks are opt-in.
- **Multi-skill repos require selection.** If a repo contains multiple `SKILL.md` files, skilltap prompts you to choose. With `--yes`, all are auto-selected.
- **npm installs use a source argument prefix.** `skilltap install skill vibe-rules` searches your taps. `skilltap install skill npm:vibe-rules` hits the npm registry.
- **Bare `skilltap` requires a TTY.** In non-TTY environments, run `skilltap status` instead.
- **`link`/`unlink`/`verify`/`enable`/`disable` are removed.** Use `adopt`, `doctor`, and `toggle` instead. Old paths print errors with hints.
- **Coming from v2.1 or earlier?** Run `skilltap migrate` to translate config and state files.

## License

MIT
