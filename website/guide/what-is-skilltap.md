---
description: CLI tool for installing agent skills from any git host. Works with Claude Code, Cursor, Codex, Gemini, and 40+ agents. Git-native, agent-agnostic, and secure.
---

# What is skilltap?

skilltap is a CLI tool for installing agent skills from any git host. Think **Homebrew taps for agent skills** -- you point it at a repo, it clones the skill, scans it for security issues, and installs it where your agents can find it.

```bash
skilltap install https://gitea.example.com/nathan/commit-helper
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

**Unified skill management.** `skilltap skills` shows every skill on your system — managed and unmanaged — across global and project scopes. Bring orphaned skills under management with `skilltap skills adopt`. Move a skill between global and project scope with `skilltap skills move`. Once adopted, skills get full source tracking, security scanning, and safe updates.

**Source-tracked updates.** skilltap remembers where every skill came from. `skilltap update` fetches upstream changes, diffs what changed, re-scans the diff, and asks before applying — you see exactly what's landing on your system.

**Multi-source taps.** Configure multiple skill indexes (taps) — your own, a friend's, a community collection. Search across all of them with `skilltap find`.

**Two-layer security scanning.** Every install runs a static scan that catches invisible Unicode, hidden HTML, obfuscated code, suspicious URLs, and tag injection attempts. Optionally run a semantic scan that uses your own agent CLI to evaluate intent.

**Standalone binary.** One file, no runtime dependencies. Download and run.

## What skilltap is NOT

**Not a package manager.** No dependency trees, no build steps, no install scripts. Skills are static files -- skilltap just puts them in the right place.

**Not a marketplace.** No centralized index, no gatekeeper. Taps are git repos anyone can create and host wherever they want — for themselves, their team, their friends, or the world.

**Not a runtime.** Skills are Markdown files that agents read. skilltap doesn't execute anything -- it clones, scans, and places files.

**Not an agent plugin system.** Some agents have their own distribution systems (Claude Code has a full plugin marketplace with commands, hooks, and MCP servers). skilltap is simpler -- it distributes SKILL.md files, and it works across all agents.

## Quick example

Install a skill from any git URL:

```bash
skilltap install https://gitea.example.com/nathan/commit-helper
```

Install with GitHub shorthand and symlink to Claude Code:

```bash
skilltap install user/commit-helper --global --also claude-code
```

Search across your configured taps:

```bash
skilltap find review
```

Install by name from a tap:

```bash
skilltap install code-reviewer
```

View all skills on your system — managed and unmanaged:

```bash
skilltap skills
```

Adopt skills you've placed manually into skilltap management:

```bash
skilltap skills adopt
```

## Next steps

- [Getting Started](./getting-started) -- install skilltap, your first skill, and adopt existing skills
- [Installing Skills](./installing-skills) -- all the ways to install, with flags and options
- [Taps](./taps) -- host your own tap and share skills with others
- [Creating Skills](./creating-skills) -- write and publish your own skills
- [Teams & Organizations](./teams) -- share skills across a team, org, or friend group with a private tap
