---
description: CLI tool for installing agent skills from any git host. Works with Claude Code, Cursor, Codex, Gemini, and 40+ agents. Git-native, agent-agnostic, and secure.
---

# What is skilltap?

skilltap is a CLI tool for installing agent skills and plugins from any git host. Think **Homebrew taps for agent skills** -- you point it at a repo, it clones the skill, scans it for security issues, and installs it where your agents can find it. When a repo contains a full plugin (skills + MCP servers + agent definitions), skilltap installs and tracks all components together.

```bash
skilltap install skill https://gitea.example.com/nathan/commit-helper
```

That's it. One command, any git host, any agent.

## The problem

The [SKILL.md format](https://agentskills.io/specification) is standardized across 40+ agents -- Claude Code, Cursor, Codex CLI, Gemini CLI, and many others. Writing skills is easy. Distributing them is not.

- **skills.sh** only indexes GitHub. If your skills live on Gitea, GitLab, or a private instance, they don't exist.
- **Agent-specific tools** like Claude Code's plugin marketplace only work within one agent. If you use multiple agents, you need multiple tools.
- **No universal installer** exists. There's no agent-agnostic, git-native way to install a skill from any source and make it available to any agent.
- **No self-hosted option.** Want to share skills with your team, your open-source project, or a group of friends? There's no standard way to run your own skill catalog without signing up for a centralized service.
- **No management layer.** Skills you've placed manually, cloned yourself, or inherited from teammates live outside any tracking system. There's no way to see what's installed, which ones are orphaned, or update them safely.

skilltap fills that gap.

## How it works

Your team maintains skills in repos on any git host. skilltap clones, scans, and installs them — then symlinks to whichever agents you use.

1. **Clone** the skill repo to a temp directory
2. **Scan** all files for security issues (invisible Unicode, hidden instructions, suspicious URLs, and more)
3. **Install** to `~/.agents/skills/` -- the universal path defined by the Agent Skills spec
4. **Symlink** to agent-specific directories if you want (`.claude/skills/`, `.cursor/skills/`, etc.)

Skills are never written to disk until they pass scanning. You always see what was found and decide whether to proceed.

## Key features

**Host your own tap.** A tap is just a git repo with a `tap.json` index — stand one up in minutes for your team, your open-source project, or a group of friends. Share the URL and anyone can subscribe. No registry account, no upload portal, no vendor lock-in.

**Git-native.** Install from any git host -- GitHub, GitLab, Gitea, Bitbucket, your company's private server. Point skilltap at any repo with a `SKILL.md` -- no special structure or manifest required. skilltap uses `git clone` under the hood, so your existing SSH keys and credential helpers just work.

**Agent-agnostic.** Installs to the universal `.agents/skills/` directory. Opt in to symlinking to agent-specific directories (Claude Code, Cursor, Codex, Gemini, Windsurf) with a single flag.

**Unified skill management.** `skilltap status` shows every skill, plugin, and MCP on your system — managed and unmanaged — across global and project scopes. Bring orphaned skills under management with `skilltap adopt`. Move a skill between global and project scope with `skilltap move <name> --scope ...`. Once adopted, skills get full source tracking, security scanning, and safe updates.

**Source-tracked updates.** skilltap remembers where every skill came from. `skilltap update` fetches upstream changes, diffs what changed, re-scans the diff, and asks before applying — you see exactly what's landing on your system.

**Multi-source taps.** Configure multiple skill indexes (taps) — your own, a friend's, a community collection. Search across all of them with `skilltap find`.

**Plugin support.** When a repo contains a plugin manifest, skilltap detects it and can install the full plugin — SKILL.md files, MCP server entries, and agent definitions — as a tracked unit. The native publish format is `.skilltap/<plugin-name>.toml` (TOML, with explicit `publish = true` opt-in). The `.claude-plugin/plugin.json` and `.codex-plugin/plugin.json` formats are also readable inputs. `skilltap status`, `skilltap info <name>`, `skilltap toggle plugin <name>[:<component>]`, and `skilltap remove plugin <name>` cover the management surface. Multi-plugin repos work via the `:plugin-name` selector (or `:*` for "install all plugins from this repo").

**Project manifest + lockfile.** Pin a project's skills, plugins, and MCP servers in `skilltap.toml` and `skilltap.lock` — `[skills]`, `[plugins]`, and `[[mcps]]` tables. Teammates check out the repo and run `skilltap sync --apply` to install the exact pinned versions. Cargo-style determinism for AI-agent skill setup.

**Read-only preview.** `skilltap try <type> <source>` (where type is `skill`, `plugin`, or `mcp`) clones, scans, and inspects a source — without writing anything to install paths or state. Useful for vetting unfamiliar sources before committing to install.

**One-shot legacy migration.** Coming from a pre-v2.2 install? Run `skilltap migrate` once to convert your state to the canonical `state.json`, translate legacy config keys (e.g. `[security.human]`, `[[security.overrides]]`, `[agent-mode]`) into the flat `[security]` + `[scanner]` blocks, and rename leftover `installed.json` / `plugins.json` to `*.v1.bak`. After migration, `loadConfig` hard-fails on any remaining legacy markers — no silent translation at runtime.

**Two-layer security scanning.** Every install runs a static scan that catches invisible Unicode, hidden HTML, obfuscated code, suspicious URLs, and tag injection attempts. Optionally run a semantic scan that uses your own agent CLI to evaluate intent.

**Non-interactive automation.** TTY detection plus `--yes` (auto-confirm) and `--json` (machine-readable output) cover AI agents, CI pipelines, and cron jobs. Set `[security] on_warn = "fail"` to hard-fail on security warnings instead of prompting. No separate "agent mode" flag, env var, or config block — every invocation is the same command surface.

**Standalone binary.** One file, no runtime dependencies. Download and run.

## What skilltap is NOT

**Not a package manager.** No dependency trees, no build steps, no install scripts. Skills are static files -- skilltap just puts them in the right place.

**Not a marketplace.** No centralized index, no gatekeeper. Taps are git repos anyone can create and host wherever they want — for themselves, their team, their friends, or the world.

**Not a runtime.** Skills are Markdown files that agents read. skilltap doesn't execute anything -- it clones, scans, and places files.

**Not a full agent plugin platform.** Some agents have their own distribution systems (Claude Code has a full plugin marketplace with commands, hooks, LSP servers, and more). skilltap handles SKILL.md files plus MCP servers and agent definitions that ship alongside a skill — the parts that are useful across agents. Agent-specific features (hooks, LSP, slash commands) are outside its scope.

## Quick example

Install a skill from any git URL:

```bash
skilltap install skill https://gitea.example.com/nathan/commit-helper
```

Install with GitHub shorthand and symlink to Claude Code:

```bash
skilltap install skill user/commit-helper --scope global --also claude-code
```

Search across your configured taps:

```bash
skilltap find review
```

Install by name from a tap:

```bash
skilltap install skill code-reviewer
```

View all skills, plugins, and MCPs on your system — managed and unmanaged:

```bash
skilltap status
```

Adopt skills you've placed manually into skilltap management:

```bash
skilltap adopt
```

## Next steps

- [Getting Started](./getting-started) -- install skilltap, your first skill, and adopt existing skills
- [Installing Skills](./installing-skills) -- all the ways to install, with flags and options
- [Taps](./taps) -- host your own tap and share skills with others
- [Creating Skills](./creating-skills) -- write and publish your own skills
- [Teams & Organizations](./teams) -- share skills across a team, org, or friend group with a private tap
