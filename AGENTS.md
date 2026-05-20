# CLAUDE.md

## Project

skilltap — CLI tool for installing agent skills (SKILL.md) and plugins from any git host. "Homebrew taps for agent skills." Installs to `.agents/skills/`, agent-agnostic, multi-source. Plugins bundle skills, MCP servers, and agent definitions as a single installable unit.

## Key Docs

Read these before making architectural decisions:
- docs/SPEC.md — exact behavior, CLI commands, file formats, algorithms, edge cases
- docs/ARCH.md — module boundaries, tech decisions, data flow
- docs/UX.md — CLI reference, flag combos, prompt flows
- docs/ROADMAP.md — current state and deferred work
- docs/VISION.md — motivation and design principles
- docs/SECURITY.md — security model

## Conventions

- **CLI surface**: `install <type> <source>` where type is `skill | plugin | mcp`. `remove <type> <name>`, `update [type] [name]`, `toggle [type] [name[:component]]`. `adopt`, `doctor`, `toggle` for adoption / verification / state changes.
- **Non-interactive**: TTY detection + `--yes` + `--json`. No `--agent` flag, no `SKILLTAP_AGENT` env var.
- **Flat `[security]` block**: `scan` (`semantic|static|none`), `on_warn` (`prompt|fail|install`), `trust` (glob array matched against tap name or source URL).
- **`[scanner]` block** (operational, separate from policy): `agent_cli`, `ollama_model`, `threshold`, `max_size`.
- **`composePolicy`** in `core/src/policy/` is the canonical resolver.
- **state.json** is the only state store. `loadConfig` hard-fails on legacy shapes pointing at `skilltap migrate`.
- **skilltap.toml + skilltap.lock** carry `[[mcps]]` + `[[mcps.lock]]` tables. Sync reconciles skills, plugins, and mcps.
- **Smart scope default**: inside a git repo, `install` defaults to `project`; outside, `global`. The inferred scope is reported in the install output.
- **`Output` interface**: all output goes through `setupOutput(args)` in CLI commands.
- **Taps are git-only.**

When adding new code, write against `state.json` directly. Do not introduce `installed.json` writes or any per-mode agent branching.

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
bun run dev                   # Run CLI from source
bun test                      # Run all tests in source mode (recursive)
bun run build                 # Compile to standalone binary (./skilltap)
bun run verify:binary         # Smoke ./skilltap (--version, --help, doctor --json)
bun run verify:binary:tests   # Build + re-run CLI test suite against compiled binary
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
bun test                       # full source-mode suite must be green
bun run build                  # compile must succeed (the released artifact path)
bun run verify:binary          # compiled binary must boot (~3 sec smoke check)
bun run verify:binary:tests    # full CLI test suite must pass against the binary
```

The verification ladder, fastest → most thorough:

1. **`bun run verify:binary`** — runs `scripts/verify-binary.sh`. Spawns `./skilltap` against an isolated temp `SKILLTAP_HOME` / `XDG_CONFIG_HOME`, asserts `--version`, `--help`, and `doctor --json` all exit 0. Catches the class of bug where `bun build --compile` succeeds but the standalone binary fails at runtime — typically caused by `--external <pkg>` flags (no `node_modules` exists inside `/$bunfs/root/`) or by accidentally bundling code that resolves a non-installed package.
2. **`bun run verify:binary:tests`** — builds the binary and re-runs the entire `packages/cli/` test suite with `SKILLTAP_TEST_BIN=$PWD/skilltap`. The test infrastructure (`runSkilltap`, `runInteractive` callers via `cliCmd()`) routes every subprocess invocation through the compiled binary instead of `bun run --bun src/index.ts`. ~80 seconds; surfaces behavioral regressions specific to the `--compile` path (dynamic imports, externals, bunfs resolution) that source-mode tests cannot see.

**Why this exists:** `bun test` and `bun run dev` cover source-mode behavior, but `--compile` is a separate code path. CI runs both layers on every push (`.github/workflows/ci.yml`). The release workflow (`.github/workflows/release.yml`) runs both the smoke and the full CLI suite against each platform's host-arch artifact (linux-x64 and darwin-arm64) before signing/upload — a release will not ship an artifact that fails the CLI suite. When you change the build script, Ink/TUI imports, or any dynamic-import boundary, run at minimum `bun run verify:binary` locally before pushing; for build-touching changes run `bun run verify:binary:tests` too.

**Routing tests through the binary in new test files:** the test infrastructure is wired via `SKILLTAP_TEST_BIN`. To make a new test file participate, use either:
- `runSkilltap(args, homeDir, configDir)` from `@skilltap/test-utils` — automatically honors the env var, and
- `cliCmd()` from `@skilltap/test-utils` for `Bun.spawn`/`runInteractive` callers — returns `[binary]` if the env var is set, else `["bun", "run", "--bun", CLI_ENTRY]`.

Do NOT hardcode `["bun", "run", "--bun", "src/index.ts"]` in new test files — that path stays bound to source forever and silently skips binary verification.

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

<!-- agile-workflow:start -->
## Agile-Workflow Substrate

Work tracked in `.work/` as markdown items with YAML frontmatter
(`kind, stage, tags, parent, depends_on, release_binding`).
Layout: `.work/active/{epics,features,stories}/`, `.work/backlog/`,
`.work/releases/<version>/`, `.work/archive/`.

**Primary query tool:** `.work/bin/work-view` filters by stage, tag, kind,
parent, and dependency. Common patterns:
- `work-view --ready` — items ready to work (deps satisfied)
- `work-view --stage review` — items waiting on user
- `work-view --parent <id>` / `--blocking <id>` — hierarchy / sequencing
- `work-view --help` for the full flag set

Detailed navigation rules in `.claude/rules/agile-workflow.md` (auto-loaded
when editing `.work/` or `docs/`). Foundation docs in `docs/` describe the
system NOW — never add legacy notes; git history is the audit trail.

### Test integrity

When running, writing, or modifying tests:

- **File real production bugs as backlog items.** When a test failure
  surfaces an actual product bug (not a stale fixture, drifted assertion,
  or broken mock), park it via `/agile-workflow:park` instead of silently
  fixing it inline mid-test-pass. The backlog item is the audit trail.
- **Fix bad tests in-session.** Stale fixtures, drifted assertions, broken
  mocks, and outdated snapshots are test debt, not product bugs. Repair
  them as you go so the suite stays meaningful.
- **Then drain small backlog bugs with a full pass.** Once tests are
  green again, if a parked production bug is small enough for a single
  stride, pick it up immediately as `/agile-workflow:scope` → design →
  implement. Larger bugs stay in backlog for prioritization.
- **NEVER game a test to make it pass.** A failing test that documents
  *why* it fails — an inline comment naming the bug, a `skip` linked to a
  backlog id, an `xfail` with a reason — is more honest than a green test
  that lies. No `expect(true).toBe(true)`, no asserting on whatever the
  code happens to return, no deleting a test as "flaky" without
  root-causing first.

Slash commands (user-invokable):
`/agile-workflow:ideate`, `/agile-workflow:epicize`,
`/agile-workflow:autopilot`, `/agile-workflow:release-deploy`.
<!-- agile-workflow:end -->
