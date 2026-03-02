# skilltap

**Install agent skills from any git host.** Homebrew taps for AI agent skills — agent-agnostic, multi-source, secure.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/nklisch/skilltap/main/install.sh | sh
```

Installs to `~/.local/bin/skilltap`. Override the install directory:

```bash
SKILLTAP_INSTALL=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/nklisch/skilltap/main/install.sh | sh
```

**Alternatives:**

```bash
bunx skilltap --help   # requires Bun
npx skilltap --help    # requires Bun on PATH
```

Or download a binary directly from [GitHub Releases](https://github.com/nklisch/skilltap/releases).

## Quickstart

```bash
# 1. Add a tap (registry of skills)
skilltap tap add my-tap https://github.com/example/my-tap

# 2. Browse available skills
skilltap find

# 3. Install a skill
skilltap install my-skill --global

# 4. List installed skills
skilltap list

# 5. Update all skills
skilltap update
```

## Commands

| Command | Description |
|---|---|
| `install <source>` | Install a skill from a URL, GitHub shorthand, npm package, or tap name |
| `remove <name>` | Remove an installed skill |
| `update [name]` | Update one or all installed skills |
| `list` | List installed skills |
| `info <name>` | Show details about a skill (installed or available in taps) |
| `find [query]` | Search skills across configured taps |
| `find --npm <query>` | Search npm registry for skills |
| `link <path>` | Link a local skill directory |
| `unlink <name>` | Remove a linked skill |
| `create [name]` | Scaffold a new skill from a template |
| `verify [path]` | Validate a skill before sharing (CI-friendly) |
| `doctor` | Check environment, config, and installed state |
| `completions <shell>` | Generate shell tab-completion script |
| `tap add <name> <url>` | Add a tap (git repo or HTTP registry) |
| `tap remove <name>` | Remove a tap |
| `tap update [name]` | Update one or all taps |
| `tap list` | List configured taps |
| `tap init <name>` | Initialize a new tap directory |
| `config` | Interactive configuration wizard |
| `config agent-mode` | Enable/disable agent mode |

Most commands accept `--global` / `--project` for scope and `--yes` to skip prompts.

## How it works

Skills are directories containing a `SKILL.md` file. skilltap installs them to `~/.agents/skills/<name>/` (global) or `.agents/skills/<name>/` (project), then creates symlinks at each agent's expected location (`.claude/skills/`, `.cursor/rules/`, etc.) so every agent picks them up automatically.

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

Enable agent mode so skilltap works headlessly from within AI agents:

```bash
skilltap config agent-mode
```

In agent mode, all prompts are suppressed, `--yes` is implied, security issues block installation with a machine-readable message, and output is plain text (no ANSI codes).

## Configuration

Config is stored at `~/.config/skilltap/config.toml`. Run the interactive wizard:

```bash
skilltap config
```

Key settings: default scope (`global`/`project`), additional agent symlinks (`--also`), security scan mode (`static`/`semantic`/`off`), and `on_warn` behavior (`block`/`warn`/`allow`).

## Authoring Skills

```bash
# Scaffold a new skill interactively
skilltap create my-skill

# Or non-interactively
skilltap create my-skill --template basic

# Edit SKILL.md, then test locally
skilltap link ./my-skill --also claude-code

# Validate before sharing
skilltap verify my-skill/

# Push to git and share
git init my-skill && cd my-skill && git add . && git commit -m "Initial skill"
git remote add origin https://github.com/you/my-skill
git push -u origin main
```

Others can install with: `skilltap install you/my-skill`

To publish to npm (with provenance), use `--template npm`. The generated GitHub Actions workflow handles publishing automatically on release.

## Trust Signals

Skills from npm show provenance status when installed:

```
$ skilltap list

Global:
  my-npm-skill   1.2.3   npm   Helpful skill   provenance
  git-skill      main    home  Another skill   curated
```

Trust tiers: `provenance` (Sigstore/SLSA verified), `publisher` (npm identity verified), `curated` (tap-verified), `unverified`.

## Shell Completions

```bash
# bash
skilltap completions bash --install

# zsh
skilltap completions zsh --install
# Add to ~/.zshrc: fpath=(~/.zfunc $fpath) && autoload -Uz compinit && compinit

# fish
skilltap completions fish --install
```

## Troubleshooting

```bash
# Check environment, config, and installed state
skilltap doctor

# Auto-repair common issues (broken symlinks, orphan records)
skilltap doctor --fix

# Machine-readable output for CI
skilltap doctor --json
```

## License

MIT
