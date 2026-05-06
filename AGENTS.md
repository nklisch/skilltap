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

## v2.1 conventions (post-cutover)

skilltap completed the v2.0 → v2.1 cutover. Most v0.x scaffolding has been retired:

- **state.json** (canonical): `~/.config/skilltap/state.json` (or `<project>/.agents/state.json`) is the canonical store for installed skills + plugins + standalone MCP servers. Written by `install`/`update`/`remove`/`disable`/`enable`/`move`/`adopt`/`link` and the `migrate` command. Read by every consumer. The legacy `installed.json` + `plugins.json` files are read-fallback only (one-time, for unmigrated v0.x users) and never written.
- **skilltap.toml + skilltap.lock** (project manifest): `install` and `remove` automatically update both when `skilltap.toml` is present at the project root. `skilltap sync` reconciles manifest ↔ lockfile ↔ state.
- **Agent mode entry points** (precedence order): `--agent` flag > `SKILLTAP_AGENT=1` env var > `[agent-mode] enabled` config block. The flag and env var work for every command via `composePolicy`. The legacy config block remains readable for back-compat.
- **Smart scope default**: inside a git repo, install defaults to `--project`; outside, `--global`. No prompt for the common case.
- **HTTP registry adapter removed** — taps are git-only. v0.x configs with `type = "http"` are silently filtered with a one-time stderr warning.

**Deferred to v2.2** (release-window concern, not a technical blocker): full deletion of v0.x read-fallback paths and `[agent-mode]` config block schema. Until then, `loadInstalled`/`loadPlugins` keep their fallback to legacy files for one-time transparent migration of unmigrated users.

When adding new code, write directly against `state.json` (`saveInstalled`/`savePlugins`/`loadInstalled` are already wired). Don't re-introduce `installed.json` writes; the dual-write layer was deleted in Refactor 2.

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
- All data boundaries validated with Zod (config, `state.json` (canonical) + `installed.json`/`plugins.json` (fallback), tap.json, plugin manifests (`.skilltap/<plugin>.toml`), `skilltap.toml` + `skilltap.lock`, frontmatter, agent responses, registry responses)
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
