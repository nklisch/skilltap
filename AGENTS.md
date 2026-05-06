# CLAUDE.md

## Project

skilltap — CLI tool for installing agent skills (SKILL.md) and plugins from any git host. "Homebrew taps for agent skills." Installs to `.agents/skills/`, agent-agnostic, multi-source. Plugins bundle skills, MCP servers, and agent definitions as a single installable unit.

## Key Docs

Read these before making architectural decisions:
- docs/SPEC.md — exact behavior, CLI commands, file formats, algorithms, edge cases (incl. v2.0 manifest + lockfile)
- docs/ARCH.md — module boundaries, tech decisions, data flow (incl. v2.0 module additions)
- docs/UX.md — CLI reference, flag combos, prompt flows
- docs/ROADMAP.md — phase plan with dependency graph (v0.1–v1.0 done; v2.0 phases 26–38)
- docs/VISION.md — motivation, design principles, v2.0 direction
- docs/SECURITY.md — security model + v2.0 simplification
- docs/PROGRESS.md — autopilot tracking: phase status, decision log, deviations
- docs/design/phase-{N}.md — per-phase design docs produced before implementation

## v2.0 conventions (in-flight transition)

skilltap is mid-transition from v0.x to v2.0. Both code paths coexist:

- **state.json** (v2.0): consolidated `~/.config/skilltap/state.json` (or `<project>/.agents/state.json`) replaces `installed.json` + `plugins.json`. Read by `skilltap status`, `skilltap doctor`. Written by `skilltap migrate`.
- **installed.json + plugins.json** (v0.x): still actively read AND written by `install`/`update`/`remove`. The cutover to v2 readers is deferred to v2.1+ (Phase 31c-c-2).
- **skilltap.toml + skilltap.lock** (v2.0): project-scope manifest + lockfile. `install` and `remove` automatically update both when `skilltap.toml` is present at the project root. `skilltap sync` reconciles manifest ↔ lockfile ↔ state.
- **`--agent` flag** (v2.0, in `composeV2`): preferred over `[agent-mode]` config block. CLI commands check both — eventually `[agent-mode]` checks should be retired.
- **Smart scope default**: inside a git repo, install defaults to `--project`; outside, `--global`. No prompt for the common case.
- **HTTP registry adapter removed** — taps are git-only. v0.x configs with `type = "http"` are silently filtered with a one-time stderr warning.

When adding new code, prefer v2.0 paths (state.json, ConfigV2, composeV2). For compatibility, keep reading both layouts where existing code does.

## Tech Stack

- **Runtime:** Bun (already on PATH — do NOT use `export PATH=...` prefixes in shell commands)
- **Language:** TypeScript (strict, ESNext, bundler module resolution)
- **CLI framework:** citty (UnJS) — see `.claude/skills/citty/SKILL.md`
- **Terminal UI:** @clack/prompts — see `.claude/skills/clack-prompts/SKILL.md`
- **Config:** TOML via smol-toml — see `.claude/skills/smol-toml/SKILL.md`
- **Validation:** Zod 4 (`import { z } from "zod/v4"`) — see `.claude/skills/zod-4/SKILL.md`
- **Security:** anti-trojan-source, out-of-character
- **Testing:** `bun test` (bun:test runner) — see `.claude/skills/bun/SKILL.md`

## Monorepo Structure

```
packages/core/    → @skilltap/core  (library, all business logic, zero CLI deps)
  src/plugin/     → plugin detection, install, lifecycle, MCP injection, state
packages/cli/     → skilltap        (CLI entry point, commands, UI)
  src/commands/plugin/  → plugin subcommands (list, info, toggle, remove)
packages/test-utils/ → @skilltap/test-utils (private, test fixtures/helpers)
```

Dependencies: `cli → core`, `cli → test-utils (dev)`, `core → test-utils (dev)`. Core never imports from cli.

## Commands

```bash
bun run dev          # Run CLI from source
bun test             # Run all tests (recursive across packages)
bun run build        # Compile to standalone binary
bun run bump <patch|minor|major|x.y.z>  # Bump version (see Versioning below)
bun test packages/core/src/plugin/      # Run plugin tests only
```

## Versioning

**Always use the bump script for version changes. Never edit version numbers by hand.**

`packages/core/package.json` is the single source of truth for the version. The `VERSION` constant exported from `@skilltap/core` is read from that file at build time. `packages/cli/package.json` must stay in lockstep — the bump script updates both atomically.

```bash
bun run bump patch   # 0.3.1 → 0.3.2
bun run bump minor   # 0.3.1 → 0.4.0
bun run bump major   # 0.3.1 → 1.0.0
bun run bump 1.2.3   # set exact version
```

After bumping, commit and tag:
```bash
git commit -am "Release v<version>"
git tag v<version>
git push --follow-tags
```

## Shell Command Rules

**NEVER:**
- Use `export PATH=...` prefixes — Bun is already on PATH
- Run `bun test` in the background (`run_in_background: true`) — it spawns dozens of processes that stay running and starves the machine

**Always run tests synchronously (foreground):**
```bash
bun test                                     # all tests — fine, run synchronously
bun test packages/core/src/doctor.test.ts   # or scoped to a file
```

## Code Conventions

### Imports
- Zod: `import { z } from "zod/v4"` — NOT `from "zod"`
- Internal: `import { thing } from "@skilltap/core"`
- Bun APIs over Node.js equivalents (Bun.$ over child_process, Bun.file over fs)

### Types
- Infer types from Zod schemas: `type Config = z.infer<typeof ConfigSchema>`
- No separate interface definitions for data shapes — Zod is the source of truth
- Interfaces only for behavior contracts (SourceAdapter, AgentAdapter)

### Error Handling
- Core functions return `Result<T, E>` — not thrown exceptions
- Error categories: UserError, GitError, ScanError, NetworkError
- Core never writes to stdout/stderr — CLI layer handles all output

### Patterns
- All data boundaries validated with Zod (config, installed.json, tap.json, frontmatter, agent responses)
- Shell out to `git` CLI directly (no git library) — user's auth just works
- Git operations go through `core/src/git.ts`
- Agent symlinks map: claude-code→.claude/skills/, cursor→.cursor/skills/, etc.

### Testing
- Use `bun:test` (`describe`, `test`, `expect`)
- Test fixtures via `@skilltap/test-utils`
- Unit tests for pure functions, integration tests for git/filesystem operations

## Git & Commits

**Do NOT add `Co-Authored-By` trailers to commit messages.** No co-author tags, no signed-off-by, no trailers of any kind. Just the commit message.

Write concise commit messages: imperative mood, focus on the "why" not the "what". One line unless a body is truly needed.

## Style

- Don't add docstrings/comments/type annotations to code you didn't change.
- Only add comments where logic isn't self-evident.
- Don't add error handling for impossible scenarios. Validate at system boundaries only.
- Prefer Bun APIs. Prefer the skills in `.claude/skills/` for API reference.
