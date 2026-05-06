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
# See what's installed (skills, plugins, taps, drift)
skilltap

# Browse skills from the built-in community tap
skilltap find

# Install a skill into the current project (auto-defaults to project scope inside a git repo)
skilltap install commit-helper --also claude-code

# Install from any git URL
skilltap install https://github.com/you/my-skill --global

# Preview a source without installing
skilltap try someone/their-skill

# View all skills (managed + unmanaged)
skilltap list

# Update all skills
skilltap update
```

## Project manifests (v2.0)

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
skilltap install nathan/commit-helper   # adds to skilltap.toml + skilltap.lock
skilltap remove commit-helper            # drops from manifest + lockfile
skilltap sync                            # show drift between manifest, lockfile, state
skilltap sync --apply                    # execute the plan
skilltap status                          # rich snapshot: skills, plugins, MCPs, drift
```

To publish a repo as a plugin (skills + MCP servers + agent definitions),
add `.skilltap/<plugin-name>.toml` with `publish = true`. See
[the v2.0 spec](docs/SPEC.md#v20--tooling-surface-redesign) for the full
manifest format.

If you've been on v0.x, run `skilltap migrate` to upgrade your global state
(`installed.json` + `plugins.json` → `state.json`) without losing anything.

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
skilltap install code-reviewer

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
skilltap install code-reviewer --global --also claude-code
skilltap update --all
```

When you update a skill in the tap, every subscriber sees the diff and confirms before it applies. Your existing SSH keys and credential helpers handle authentication. See [Host Your Own Tap](https://skilltap.dev/guide/teams) for the full setup guide and config options.

## Commands

| Command | Description |
|---|---|
| `(no args)` | Status dashboard — installed skills, plugins, MCPs, drift |
| `status` | Same as bare invocation, with `--json` for scripting |
| `install <source>` | Install a skill from a URL, GitHub shorthand, npm package, or tap name |
| `remove <name>` | Remove an installed skill |
| `update [name]` | Update one or all installed skills |
| `list` | List installed skills |
| `info <name>` | Show details about a skill (installed or available in taps) |
| `find [query]` | Search skills across configured taps |
| `try <source>` | Preview a source (clone, parse, scan) without installing |
| `sync` | Show drift between `skilltap.toml`, `skilltap.lock`, and installed state |
| `sync --apply` | Execute the sync plan via install/remove |
| `migrate` | One-shot upgrade from v0.x state to v2.0 |
| `link <path>` | Link a local skill directory |
| `unlink <name>` | Remove a linked skill |
| `toggle <plugin>[:component]` | Toggle a plugin component (or open picker) |
| `enable <plugin>[:component]` | Activate a plugin component (or all inactive) |
| `disable <plugin>[:component]` | Deactivate a plugin component (or all active) |
| `create [name]` | Scaffold a new skill from a template |
| `verify [path]` | Validate a skill before sharing (CI-friendly) |
| `doctor` | Check environment, config, manifest/lockfile drift, MCP consistency |
| `completions <shell>` | Generate shell tab-completion script |
| `tap add <name> <url>` | Add a git tap |
| `tap remove <name>` | Remove a tap |
| `tap update [name]` | Update one or all taps |
| `tap list` | List configured taps |
| `tap init <name>` | Initialize a new tap directory |
| `config` | Interactive configuration wizard |
| `config agent-mode` | Enable/disable agent mode (legacy; see Agent flag below) |

Most commands accept `--global` / `--project` for scope, `--yes` to skip prompts,
and `--agent` for non-interactive use. **Smart scope default**: inside a git repo,
`install` defaults to `--project`; outside, `--global`. Override explicitly with
`--global` / `--project` whenever you need to.

## How it works

Skills are directories containing a `SKILL.md` file. skilltap installs them to `~/.agents/skills/<name>/` (global) or `.agents/skills/<name>/` (project), then creates symlinks at each agent's expected location (`.claude/skills/`, `.cursor/skills/`, etc.) so every agent picks them up automatically.

## Security

Every install and update runs a two-layer security scan before anything lands on disk.

**Static scan** (always on by default): checks for invisible Unicode, hidden HTML/CSS, markdown injection, obfuscated code, suspicious URLs, dangerous shell patterns, and tag injection.

**Semantic scan** (optional, `--semantic`): sends skill content to your local AI agent in bounded 2000-char chunks. The agent is invoked with tools disabled (`--no-tools`), so even a skill that tricks the reviewer can't cause it to take actions. Content is wrapped in a randomly-suffixed untrusted block so the agent can't be hijacked by the skill it's reviewing. Closing tags that could escape the wrapper are detected and escaped before the chunk is sent. Up to 4 chunks are evaluated in parallel; agent failures are fail-open (scan continues).

```bash
skilltap install my-skill --semantic   # enable semantic scan
skilltap install my-skill --strict     # block on any warning
skilltap install my-skill --skip-scan  # bypass scanning (trusted sources)
```

See [docs/SECURITY.md](docs/SECURITY.md) for the full threat model, detector reference, and configuration options.

## Agent mode

For non-interactive use (AI agents, CI, scripts), three options that compose:

```bash
skilltap install foo --agent             # one-off flag (v2.0)
SKILLTAP_AGENT=1 skilltap install foo    # env var (v2.0)
skilltap config agent-mode               # interactive wizard, sticky (v0.x; still works)
```

`--agent` (or `SKILLTAP_AGENT=1`) suppresses all prompts, implies `--yes`,
turns security warnings into hard failures, and emits plain text output. Agents
should set the flag or env var on every invocation.

## Configuration

Config is stored at `~/.config/skilltap/config.toml`. Run the interactive wizard:

```bash
skilltap config
```

Key settings: default scope (`global`/`project`), additional agent symlinks (`--also`), security scan mode (`static`/`semantic`/`off`), and `on_warn` behavior (`prompt`/`fail`).

## Authoring Skills

```bash
# Scaffold a new skill interactively
skilltap create my-skill

# Edit SKILL.md, then test locally
skilltap link ./my-skill --also claude-code

# Validate before sharing
skilltap verify my-skill/

# Push to git and share
git push -u origin main
```

Others can install with: `skilltap install you/my-skill`

To publish to npm (with provenance), use `--template npm`. The generated GitHub Actions workflow handles publishing automatically on release.

## Trust Signals

Skills from npm show provenance status when installed:

```
$ skilltap skills

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

- **`--yes` does not skip the scope prompt.** Pass `--global` or `--project` explicitly for a fully non-interactive install.
- **`--yes` does not bypass security warnings.** Use `--strict` to turn warnings into hard failures, or `--skip-scan` to bypass entirely (blocked if `require_scan = true`).
- **Agent mode must be enabled before invoking from an AI agent.** Run `skilltap config agent-mode` interactively once. Without it, skilltap will prompt and hang in non-TTY environments.
- **Agent symlinks are not automatic.** Pass `--also <agent>` or set defaults in config. The skill always lands in `.agents/skills/` — symlinks are opt-in.
- **Multi-skill repos require selection.** If a repo contains multiple `SKILL.md` files, skilltap prompts you to choose. With `--yes`, all are auto-selected.
- **npm installs require the `npm:` prefix.** `skilltap install vibe-rules` searches your taps. `skilltap install npm:vibe-rules` hits the npm registry.

## License

MIT
