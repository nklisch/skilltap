# Roadmap

Implementation plan for skilltap v0.1 — derived from VISION.md, ARCH.md, UX.md, and SPEC.md.

## Phase 0 — Project Scaffolding

Set up the monorepo, tooling, and build pipeline before writing any feature code.

- [ ] **0.1** Initialize Bun workspace root (`package.json`, `bunfig.toml`, `tsconfig.json` base)
- [ ] **0.2** Create `packages/core/` — `package.json` (`@skilltap/core`), `tsconfig.json`, `src/` skeleton
- [ ] **0.3** Create `packages/cli/` — `package.json` (`skilltap`), `tsconfig.json`, `src/index.ts` entry with citty `runMain`
- [ ] **0.4** Create `packages/test-utils/` — `package.json` (`@skilltap/test-utils`, private), `tsconfig.json`, placeholder exports
- [ ] **0.5** Wire workspace dependencies: `cli → core`, `cli → test-utils (dev)`, `core → test-utils (dev)`
- [ ] **0.6** Install shared deps: `zod@4`, `smol-toml`, `@clack/prompts`, `citty`
- [ ] **0.7** Install security deps: `anti-trojan-source`, `out-of-character`
- [ ] **0.8** Verify `bun run` / `bun test` works across all three packages
- [ ] **0.9** Add root scripts: `dev` (run CLI from source), `test` (all packages), `build` (compile CLI)

**Exit criteria:** `bun run dev -- --help` prints the skilltap command tree stub. `bun test` passes with a placeholder test per package.

---

## Phase 1 — Core Types, Schemas, and Config

Build the data layer that everything else sits on. No I/O except config file read/write.

- [ ] **1.1** Define `Result<T, E>` type and error categories (`UserError`, `GitError`, `ScanError`, `NetworkError`) in `core/src/types.ts`
- [ ] **1.2** Define Zod schemas in `core/src/schemas/`:
  - `config.ts` — `ConfigSchema`, `SecurityConfigSchema`, `AgentModeSchema`
  - `installed.ts` — `InstalledJsonSchema`, `InstalledSkillSchema`
  - `tap.ts` — `TapSchema`, `TapSkillSchema`
  - `skill.ts` — `SkillFrontmatterSchema`
  - `agent.ts` — `AgentResponseSchema`, `ResolvedSourceSchema`
- [ ] **1.3** Implement `core/src/config.ts`:
  - `loadConfig()` — read `~/.config/skilltap/config.toml`, parse with smol-toml, validate with Zod, return `Result<Config>`
  - `saveConfig()` — serialize and write
  - `loadInstalled()` / `saveInstalled()` — read/write `installed.json`
  - `ensureDirs()` — create `~/.config/skilltap/`, `taps/`, `cache/` on first run
  - Default config creation when file is missing
- [ ] **1.4** Unit tests for all schemas (valid/invalid fixtures) and config round-trip

**Exit criteria:** Config can be created, read, modified, and written. All schemas validate correctly against example data from SPEC.md.

---

## Phase 2 — Git Operations and Skill Discovery

The two foundation modules for the install flow.

- [ ] **2.1** Implement `core/src/git.ts`:
  - `clone(url, dest, opts)` — shallow clone (`--depth 1`), optional `--branch`
  - `pull(dir)`, `fetch(dir)`, `diff(dir, from, to)`
  - `revParse(dir)` — current HEAD SHA
  - `log(dir, n)` — last n commit summaries
  - All functions return `Result<T, GitError>`
  - Temp dir helper (create in `/tmp/skilltap-{random}/`, clean up)
- [ ] **2.2** Implement `core/src/scanner.ts`:
  - Scan algorithm per SPEC: root SKILL.md → `.agents/skills/*/SKILL.md` → agent-specific paths → deep scan
  - SKILL.md frontmatter parsing (YAML between `---` delimiters)
  - Validate with `SkillFrontmatterSchema`, fallback to directory name
  - Deduplication by name (prefer `.agents/skills/` path)
  - Returns `{ name, description, path, valid, warnings }[]`
- [ ] **2.3** Set up `packages/test-utils/`:
  - `fixtures.ts` — create mock skill repos (standalone, multi-skill) as temp git repos
  - `git.ts` — `initRepo()`, `commitAll()` helpers
  - `tmp.ts` — temp directory lifecycle
  - Static fixtures: `standalone-skill/`, `multi-skill-repo/`, `sample-tap/`
- [ ] **2.4** Integration tests: clone fixture repos, scan for skills, verify discovery results

**Exit criteria:** Can clone a git repo to a temp dir, scan it, and get back a typed list of discovered skills with parsed frontmatter.

---

## Phase 3 — Source Adapters

Resolve user input into cloneable URLs.

- [ ] **3.1** Define `SourceAdapter` interface in `core/src/adapters/types.ts`
- [ ] **3.2** Implement `core/src/adapters/git.ts` — handles `https://`, `git@`, `ssh://` URLs (pass-through)
- [ ] **3.3** Implement `core/src/adapters/github.ts` — handles `github:owner/repo` and bare `owner/repo` shorthand
- [ ] **3.4** Implement `core/src/adapters/local.ts` — handles `./`, `/`, `~/` paths, validates SKILL.md exists
- [ ] **3.5** Source resolution router: try adapters in SPEC order (URL → github: → local → shorthand → tap name)
- [ ] **3.6** Unit tests for each adapter's `canHandle()` and `resolve()`

**Exit criteria:** Any source string from SPEC (URLs, shorthand, local paths) resolves to a `ResolvedSource` validated by Zod.

---

## Phase 4 — Install and Remove

The core install/remove flow without security scanning (added next phase).

- [ ] **4.1** Implement `core/src/install.ts`:
  - `installSkill(source, options)` — full orchestration: resolve → clone → scan → select → place → record
  - Standalone repo handling: move entire clone to install dir
  - Multi-skill repo handling: clone to cache, copy selected skill dirs to install dir
  - Scope resolution (global vs project, with project root detection via `.git` walk)
  - Write to `installed.json`
  - "Already installed" detection
- [ ] **4.2** Implement `core/src/symlink.ts`:
  - `createAgentSymlinks(skillName, scope, agents)` — create symlinks for each agent identifier
  - `removeAgentSymlinks(skillName, scope, agents)` — clean up
  - Agent path mapping per SPEC (claude-code → `.claude/skills/`, etc.)
  - Parent directory creation
- [ ] **4.3** Implement `removeSkill(name, options)` in install.ts:
  - Remove agent symlinks first
  - Remove skill directory
  - Remove cache entry if last skill from multi-skill repo
  - Update `installed.json`
- [ ] **4.4** Integration tests: install standalone skill, install from multi-skill repo, remove, verify filesystem state and `installed.json`

**Exit criteria:** Can install a skill from a git URL (standalone and multi-skill), create agent symlinks, remove it, and track state in `installed.json`. No security scanning yet.

---

## Phase 5 — Security Scanning (Layer 1 — Static)

Pattern-matching scanner that runs on every install.

- [ ] **5.1** Implement `core/src/security/patterns.ts`:
  - Invisible Unicode detection (via `anti-trojan-source`, `out-of-character`)
  - Hidden HTML/CSS patterns (regex)
  - Markdown hiding patterns
  - Obfuscation detection (base64, hex, data URIs, variable expansion)
  - Suspicious URL list (ngrok, webhook.site, requestbin, etc.)
  - Dangerous shell/env patterns
  - Tag injection patterns
- [ ] **5.2** Implement `core/src/security/static.ts`:
  - `scanStatic(dir)` — scan all files in skill directory
  - File type checks (binaries, compiled, minified)
  - Size checks (total dir, per-file)
  - Returns `{ file, line, category, raw, visible, decoded }[]`
- [ ] **5.3** Wire scanning into install flow (between clone and place)
- [ ] **5.4** Build `malicious-skill/` test fixture with known-bad patterns from SPEC
- [ ] **5.5** Unit tests: every detection category has at least one positive and one negative test case
- [ ] **5.6** Integration test: install a malicious fixture, verify all warnings are surfaced

**Exit criteria:** Every pattern category from SPEC is detected. Clean skills pass with zero warnings. Known-bad fixtures produce correct, attributed warnings.

---

## Phase 6 — CLI Commands (Core Set)

Wire core logic to CLI commands with interactive UI.

- [ ] **6.1** Set up citty command structure in `cli/src/commands/`:
  - `install.ts`, `remove.ts`, `list.ts`, `info.ts`, `link.ts`, `unlink.ts`
- [ ] **6.2** Implement `cli/src/ui/`:
  - `format.ts` — table formatting, colors, terminal width handling
  - `prompts.ts` — @clack/prompts wrappers for skill selection, scope selection, install confirmation
  - `scan.ts` — security warning display (formatted per SPEC)
- [ ] **6.3** `skilltap install` — full interactive flow:
  - Source resolution, skill selection (single auto / multi prompt / `--yes` auto-all)
  - Scope prompt (unless `--project` / `--global`)
  - Security scan display and prompts
  - `--strict`, `--skip-scan`, `--also`, `--ref`, `--yes` flags
  - All flag combinations from UX.md decision matrix
- [ ] **6.4** `skilltap remove` — confirm prompt, `--yes`, `--project`
- [ ] **6.5** `skilltap list` — global/project grouping, `--json`, empty state
- [ ] **6.6** `skilltap info` — installed/available/linked/not-found states
- [ ] **6.7** `skilltap link` / `skilltap unlink` — symlink creation, no prompts
- [ ] **6.8** CLI tests: snapshot tests for output formatting, mock core functions

**Exit criteria:** Can run `skilltap install <url>`, walk through all prompts, see security warnings, install with symlinks, list, info, remove. All flag combos work per UX.md.

---

## Phase 7 — Tap Management

Add tap support — the curated index model.

- [ ] **7.1** Implement `core/src/taps.ts`:
  - `addTap(name, url)` — clone to `~/.config/skilltap/taps/{name}/`, validate `tap.json`, update config
  - `removeTap(name)` — remove dir, update config
  - `updateTap(name?)` — git pull one or all taps
  - `loadTaps()` — parse all `tap.json` files, return merged skill list
  - `searchTaps(query)` — fuzzy match against name, description, tags
- [ ] **7.2** Wire tap name resolution into install flow (source resolution step 5-6 per SPEC)
- [ ] **7.3** CLI commands in `cli/src/commands/tap/`:
  - `tap add`, `tap remove`, `tap list`, `tap update`, `tap init`
- [ ] **7.4** `skilltap find` — search across taps, `--json`, empty/no-results states
- [ ] **7.5** `skilltap find -i` — interactive fuzzy finder
- [ ] **7.6** `skilltap install <name>` — resolve from taps, handle single/multiple matches
- [ ] **7.7** `skilltap install <name>@<ref>` — version pinning via tap resolution
- [ ] **7.8** Integration tests: add tap fixture, search, install by name, tap update
- [ ] **7.9** Create `sample-tap/tap.json` test fixture

**Exit criteria:** Full tap lifecycle works. Can add a tap, search it, install by skill name, update taps. `skilltap find` returns results across multiple taps.

---

## Phase 8 — Update Flow

Diff-aware updates with security re-scanning.

- [ ] **8.1** Implement update logic in `core/src/install.ts`:
  - `updateSkill(name?)` — fetch, compare SHAs, compute diff
  - Standalone: `git pull` directly
  - Multi-skill: pull cache repo, re-copy skill dir
  - Scan changed content only (diff-based static scan)
  - Update `installed.json` (new SHA, `updatedAt`)
  - Skip linked skills
- [ ] **8.2** CLI `skilltap update [name]`:
  - Per-skill diff summary (files changed, insertions, deletions)
  - Security scan on diff
  - `--yes` (auto-accept clean), `--strict` (skip on warnings)
  - Summary line: `Updated: N  Skipped: N  Up to date: N`
- [ ] **8.3** Integration tests: modify fixture repo, run update, verify new content and state

**Exit criteria:** `skilltap update` detects changes, shows diffs, scans changed content, applies cleanly. Linked skills are skipped.

---

## Phase 9 — Security Scanning (Layer 2 — Semantic)

Agent-based evaluation for deeper analysis.

- [ ] **9.1** Implement agent adapters in `core/src/agents/`:
  - `types.ts` — `AgentAdapter` interface
  - `detect.ts` — scan PATH for known agent binaries
  - `claude.ts` — Claude Code adapter (`claude --print -p ... --no-tools --output-format json`)
  - `gemini.ts` — Gemini CLI adapter
  - `codex.ts` — Codex CLI adapter
  - `opencode.ts` — OpenCode adapter
  - `ollama.ts` — Ollama adapter (model listing, selection)
- [ ] **9.2** JSON extraction pipeline (`core/src/security/semantic.ts`):
  - Direct parse → code block extraction → regex extraction → Zod validation
  - Fail-open with warning on parse failure
- [ ] **9.3** Implement chunking algorithm:
  - Concatenate text files, split at 200-500 tokens (~800-2000 chars)
  - Prefer paragraph boundaries, then sentence, then hard split
  - Retain source file + line range per chunk
- [ ] **9.4** Pre-scan chunks for tag injection, escape and auto-flag (risk 10/10)
- [ ] **9.5** Security prompt template with randomized wrapper tags (per SPEC)
- [ ] **9.6** Parallel chunk evaluation (max 4 concurrent)
- [ ] **9.7** Score aggregation, threshold filtering, sorted output
- [ ] **9.8** Wire into install/update flows:
  - Trigger conditions: config `scan = "semantic"`, `--semantic` flag, or user accepts "Run semantic scan?" prompt
  - First-use agent selection flow (interactive, saves to config)
- [ ] **9.9** Unit tests: chunking, JSON extraction, tag injection escaping
- [ ] **9.10** Integration test with mock agent (return known scores, verify aggregation)

**Exit criteria:** Semantic scan chunks content, invokes a real or mock agent, aggregates scores, and surfaces flagged chunks with attribution. First-use agent selection works.

---

## Phase 10 — Config Wizard and Agent Mode

Interactive setup and the agent-safety layer.

- [ ] **10.1** `skilltap config` wizard (`cli/src/commands/config/index.ts`):
  - Scope, agent symlinks, scan level, agent selection, warning behavior
  - `--reset` flag with confirmation
  - Writes to `config.toml`
- [ ] **10.2** `skilltap config agent-mode` wizard (`cli/src/commands/config/agent-mode.ts`):
  - TTY check (reject non-interactive)
  - Enable/disable flow
  - Scope, agent symlinks, scan level for agent installs
  - Write `[agent-mode]` section to config
- [ ] **10.3** Agent mode runtime behavior:
  - Detect `agent-mode.enabled` on startup
  - Force: `yes=true`, `on_warn="fail"`, `require_scan=true`
  - Plain text output (no ANSI, no spinners) — `cli/src/ui/agent-out.ts`
  - Security failure directive message (per SPEC)
  - Block `--skip-scan`, block config overrides
- [ ] **10.4** Security policy composition logic:
  - Config + CLI flags compose, most restrictive wins
  - `--strict` / `--no-strict` override `on_warn`
  - `require_scan` blocks `--skip-scan`
  - Agent mode overrides everything
- [ ] **10.5** Tests: agent mode output format, TTY rejection, policy composition matrix

**Exit criteria:** `skilltap config` generates valid config. Agent mode produces plain text, blocks security bypasses, emits stop directives on warnings.

---

## Phase 11 — Polish, Edge Cases, Build

Finalize for release.

- [ ] **11.1** Error messages and hints for all conditions in SPEC error table
- [ ] **11.2** `--json` output for `list`, `find`, `info`
- [ ] **11.3** Terminal width handling (truncate descriptions, responsive tables)
- [ ] **11.4** Empty state messages for all commands (no skills, no taps, no results)
- [ ] **11.5** `bun build --compile` — produce standalone binary, test on clean machine
- [ ] **11.6** npm publish setup: `skilltap` (cli) and `@skilltap/core` packages
- [ ] **11.7** `bunx skilltap` / `npx skilltap` verification
- [ ] **11.8** End-to-end test: fresh config → add tap → find → install → list → update → remove
- [ ] **11.9** README with quickstart (install, add tap, install skill)

**Exit criteria:** `skilltap` is installable via `bunx`, `npx`, or standalone binary. All v0.1 features from SPEC work end-to-end.

---

## Dependency Graph

```
Phase 0 (scaffolding)
  └→ Phase 1 (types, schemas, config)
       ├→ Phase 2 (git, scanner)
       │    ├→ Phase 3 (source adapters)
       │    │    └→ Phase 4 (install/remove)
       │    │         ├→ Phase 5 (static security)
       │    │         │    └→ Phase 6 (CLI commands)
       │    │         │         ├→ Phase 7 (taps)
       │    │         │         │    └→ Phase 8 (update)
       │    │         │         └→ Phase 9 (semantic security)
       │    │         │              └→ Phase 10 (config wizard, agent mode)
       │    │         │                   └→ Phase 11 (polish, build)
       │    │         └→ Phase 6 (partial — link/unlink/list don't need security)
       │    └→ Phase 5 (patterns are scanner-independent, can parallel)
       └→ Phase 3 (adapters only need schemas, not git)
```

Phases 5 and 3 can be developed in parallel after Phase 2. Phase 6 can start partially (list, link, unlink) as soon as Phase 4 is done, before security scanning is wired in.

---

## What's Deferred to v0.2+

Not in scope for this roadmap (per SPEC version scope):

- npm adapter (`npm:@scope/name`)
- HTTP registry adapter + endpoints
- Standalone binary distribution (Homebrew formula)
- Shell completions (bash, zsh, fish)
- `skilltap doctor`
- Community trust signals (`verified`, `reviewedBy`)
- `skilltap publish`
- Skill templates (`skilltap create`)
- Windows support
