# CLAUDE.md

## Project

skilltap — CLI tool for installing agent skills (SKILL.md) from any git host. "Homebrew taps for agent skills." Installs to `.agents/skills/`, agent-agnostic, multi-source.

## Key Docs

Read these before making architectural decisions:
- docs/SPEC.md — exact behavior, CLI commands, file formats, algorithms, edge cases
- docs/ARCH.md — module boundaries, tech decisions, data flow
- docs/UX.md — CLI reference, flag combos, prompt flows
- docs/ROADMAP.md — 11-phase implementation plan with dependency graph
- docs/VISION.md — motivation, design principles
- docs/SECURITY.md — two-layer security model, threat model, chunking strategy

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
packages/cli/     → skilltap        (CLI entry point, commands, UI)
packages/test-utils/ → @skilltap/test-utils (private, test fixtures/helpers)
```

Dependencies: `cli → core`, `cli → test-utils (dev)`, `core → test-utils (dev)`. Core never imports from cli.

## Commands

```bash
bun run dev          # Run CLI from source
bun test             # Run all tests (recursive across packages)
bun run build        # Compile to standalone binary
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
