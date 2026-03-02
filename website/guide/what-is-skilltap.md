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

skilltap fills that gap.

## How it works

```
Git host (any)          skilltap           Your machine
┌──────────┐           ┌────────┐         ┌────────────────────┐
│ Gitea    │──clone──▶ │ scan   │──────▶  │ ~/.agents/skills/  │
│ GitHub   │           │ install│         │                    │
│ GitLab   │           │ link   │         │ optional symlinks: │
│ Your own │           └────────┘         │ ~/.claude/skills/  │
└──────────┘                              │ ~/.cursor/skills/  │
                                          └────────────────────┘
```

1. **Clone** the skill repo to a temp directory
2. **Scan** all files for security issues (invisible Unicode, hidden instructions, suspicious URLs, and more)
3. **Install** to `~/.agents/skills/` -- the universal path defined by the Agent Skills spec
4. **Symlink** to agent-specific directories if you want (`.claude/skills/`, `.cursor/skills/`, etc.)

Skills are never written to disk until they pass scanning. You always see what was found and decide whether to proceed.

## Key features

**Git-native.** Install from any git host -- GitHub, GitLab, Gitea, Bitbucket, your company's private server. Point skilltap at any repo with a `SKILL.md` -- no special structure or manifest required. skilltap uses `git clone` under the hood, so your existing SSH keys and credential helpers just work.

**Agent-agnostic.** Installs to the universal `.agents/skills/` directory. Opt in to symlinking to agent-specific directories (Claude Code, Cursor, Codex, Gemini, Windsurf) with a single flag.

**Multi-source taps.** Configure multiple skill indexes (taps) -- your own, a friend's, a community collection. Search across all of them with `skilltap find`.

**Two-layer security scanning.** Every install runs a static scan that catches invisible Unicode, hidden HTML, obfuscated code, suspicious URLs, and tag injection attempts. Optionally run a semantic scan that uses your own agent CLI to evaluate intent.

**Standalone binary.** One file, no runtime dependencies. Download and run.

## What skilltap is NOT

**Not a package manager.** No dependency trees, no build steps, no install scripts. Skills are static files -- skilltap just puts them in the right place.

**Not a marketplace.** No centralized index. Taps are git repos anyone can create and host wherever they want.

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

## Next steps

- [Getting Started](./getting-started) -- install skilltap and your first skill
- [Installing Skills](./installing-skills) -- all the ways to install, with flags and options
- [Creating Skills](./creating-skills) -- write and publish your own skills
