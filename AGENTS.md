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
- docs/designs/completed/phase-{N}.md — per-phase design docs produced before implementation

## v2.0 Redesign conventions

The v2.0 redesign (Phases 39–46) is complete. Key conventions for new code:

- **CLI surface**: `install <type> <source>` where type is `skill | plugin | mcp`. No auto-detect. `remove <type> <name>`, `update [type] [name]`, `toggle [type] [name[:component]]`. No `link`, `unlink`, `verify`, `enable`, `disable` commands — use `adopt`, `doctor`, `toggle`.
- **No `--agent` flag, no `SKILLTAP_AGENT` env var**: removed entirely. TTY detection + `--yes` + `--json` drives non-interactive behavior. There is no `[agent-mode]` config block. `composePolicy` no longer has an agent-mode branch.
- **Flat `[security]` block**: `scan`, `on_warn`, `require_scan`, `agent_cli`, `threshold`, `max_size`, `ollama_model`, `overrides` (array of `{match, kind, preset}`) — no `[security.human]`/`[security.agent]` per-mode split. The `trust = [...]` glob design and `policy-v2/` module are unshipped scaffolding (see Gaps section). `composePolicy` reads `config.security.overrides` and resolves the `preset` (`none`/`relaxed`/`standard`/`strict`) to concrete `scan`/`on_warn`/`require_scan` values.
- **state.json** (canonical): `~/.config/skilltap/state.json` (or `<project>/.agents/state.json`) is the only state store. Written by `install`/`update`/`remove`/`move`/`adopt`/`toggle`/`migrate`. No legacy `installed.json`/`plugins.json` fallback — `migrate` is the explicit upgrade path.
- **skilltap.toml + skilltap.lock** (project manifest): `install` and `remove` update both when `skilltap.toml` is present. `skilltap sync` reconciles manifest ↔ lockfile ↔ state.
- **Smart scope default**: inside a git repo, install defaults to `project`; outside, `global`. Use `--scope project|global` to override.
- **Output interface**: all output goes through `Output` (from `core/src/output/types.ts`). Core functions never write to stdout/stderr. `setupOutput(args)` in CLI commands wires the concrete implementation.
- **HTTP registry adapter removed** — taps are git-only.
- **No silent aliases**: old command paths return errors with hints. Don't add aliases.

When adding new code, write against `state.json` directly. Don't re-introduce `installed.json` writes or any per-mode agent branching.

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
  src/agent-plugins/ → AgentPluginScanner interface + Claude Code adapter
  src/output/     → Output interface (tty/plain/json mode abstraction)
packages/cli/     → skilltap        (CLI entry point, commands, UI)
  src/commands/install/  → skill.ts, plugin.ts, mcp.ts subcommands
  src/commands/remove/   → skill.ts, plugin.ts, mcp.ts subcommands
  src/tui/        → Ink-based TUI dashboard + screens
packages/test-utils/ → @skilltap/test-utils (private, test fixtures/helpers)
```

Dependencies: `cli → core`, `cli → test-utils (dev)`, `core → test-utils (dev)`. Core never imports from cli.

## Commands

```bash
bun run dev             # Run CLI from source
bun test                # Run all tests (recursive across packages)
bun run build           # Compile to standalone binary (./skilltap)
bun run verify:binary   # Boot-test ./skilltap (--version, --help, doctor --json)
bun run bump <patch|minor|major|x.y.z>  # Bump version (see Versioning below)
bun test packages/core/src/plugin/      # Run plugin tests only

scripts/install-local.sh           # Build + install ./skilltap to ~/.local/bin
scripts/install-local.sh --link    # Symlink instead of copy (live rebuild updates)
scripts/verify-binary.sh --build   # Build then smoke-test in one step
scripts/verify-binary.sh skilltap-linux-x64  # Smoke a specific binary path
```

## Versioning & Release Verification

**Always use the bump script for version changes. Never edit version numbers by hand.**

`packages/core/package.json` is the single source of truth for the version. The `VERSION` constant exported from `@skilltap/core` is read from that file at build time. `packages/cli/package.json` must stay in lockstep — the bump script updates both atomically.

```bash
bun run bump patch   # 0.3.1 → 0.3.2
bun run bump minor   # 0.3.1 → 0.4.0
bun run bump major   # 0.3.1 → 1.0.0
bun run bump 1.2.3   # set exact version
```

### Pre-release checklist (run before tagging)

```bash
bun test                       # full suite must be green
bun run build                  # compile must succeed (the released artifact path)
bun run verify:binary          # compiled binary must boot and run core commands
```

`bun run verify:binary` runs `scripts/verify-binary.sh`, which:
- Boots `./skilltap` against an isolated temp `SKILLTAP_HOME` / `XDG_CONFIG_HOME`
- Asserts `--version`, `--help`, and `doctor --json` all exit 0 with expected output
- Catches the class of bug where `bun build --compile` succeeds but the standalone binary fails at runtime — typically caused by `--external <pkg>` flags (no `node_modules` exists inside `/$bunfs/root/`) or by accidentally bundling code that resolves a non-installed package

**Why this exists:** `bun test` and `bun run dev` cover source-mode behavior, but `--compile` is a separate code path. CI runs the verifier on every push (`.github/workflows/ci.yml`) and on the release host-arch binary before it gets uploaded (`.github/workflows/release.yml`). When you change the build script, Ink/TUI imports, or any dynamic-import boundary, run `bun run verify:binary` locally before pushing.

**Adding new build-time dependencies:** if you need a package only in development mode (`process.env.DEV === 'true'`) or a similarly-gated path, *do not* mark it `--external` in the `--compile` build. The compiled binary cannot resolve externals at runtime. Either (a) install it as a regular dependency and let it bundle, or (b) lazy-load it via `await import()` and ensure its package is on the `dependencies` list so bun finds it during compile.

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
- All data boundaries validated with Zod (config, `state.json`, tap.json, plugin manifests (`.skilltap/<plugin>.toml`), `skilltap.toml` + `skilltap.lock`, frontmatter, agent responses, registry responses)
- Shell out to `git` CLI directly (no git library) — user's auth just works
- Git operations go through `core/src/git.ts`
- Agent symlinks map: claude-code→.claude/skills/, cursor→.cursor/skills/, etc.
- Output goes through `Output` interface (`setupOutput(args)` in CLI commands) — never `process.stdout.write` directly from command handlers

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
