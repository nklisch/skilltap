# Roadmap

Implementation plan for skilltap — derived from VISION.md, ARCH.md, UX.md, and SPEC.md.

## v0.1 — Core + Taps (complete)

### Phase 0 — Project Scaffolding ✓

Set up the monorepo, tooling, and build pipeline before writing any feature code.

- [x] **0.1** Initialize Bun workspace root (`package.json`, `bunfig.toml`, `tsconfig.json` base)
- [x] **0.2** Create `packages/core/` — `package.json` (`@skilltap/core`), `tsconfig.json`, `src/` skeleton
- [x] **0.3** Create `packages/cli/` — `package.json` (`skilltap`), `tsconfig.json`, `src/index.ts` entry with citty `runMain`
- [x] **0.4** Create `packages/test-utils/` — `package.json` (`@skilltap/test-utils`, private), `tsconfig.json`, placeholder exports
- [x] **0.5** Wire workspace dependencies: `cli → core`, `cli → test-utils (dev)`, `core → test-utils (dev)`
- [x] **0.6** Install shared deps: `zod@4`, `smol-toml`, `@clack/prompts`, `citty`
- [x] **0.7** Install security deps: `anti-trojan-source`, `out-of-character`
- [x] **0.8** Verify `bun run` / `bun test` works across all three packages
- [x] **0.9** Add root scripts: `dev` (run CLI from source), `test` (all packages), `build` (compile CLI)

---

### Phase 1 — Core Types, Schemas, and Config ✓

Build the data layer that everything else sits on. No I/O except config file read/write.

- [x] **1.1** Define `Result<T, E>` type and error categories (`UserError`, `GitError`, `ScanError`, `NetworkError`) in `core/src/types.ts`
- [x] **1.2** Define Zod schemas in `core/src/schemas/`
- [x] **1.3** Implement `core/src/config.ts` (load/save config, installed.json, ensureDirs)
- [x] **1.4** Unit tests for all schemas (valid/invalid fixtures) and config round-trip

---

### Phase 2 — Git Operations and Skill Discovery ✓

The two foundation modules for the install flow.

- [x] **2.1** Implement `core/src/git.ts` (clone, pull, fetch, diff, revParse, log)
- [x] **2.2** Implement `core/src/scanner.ts` (SKILL.md discovery, frontmatter parsing, deduplication)
- [x] **2.3** Set up `packages/test-utils/` (fixtures, git helpers, temp dir management)
- [x] **2.4** Integration tests: clone fixture repos, scan for skills, verify discovery results

---

### Phase 3 — Source Adapters ✓

Resolve user input into cloneable URLs.

- [x] **3.1** Define `SourceAdapter` interface in `core/src/adapters/types.ts`
- [x] **3.2** Implement git adapter (https, git@, ssh URLs)
- [x] **3.3** Implement GitHub adapter (github:owner/repo, bare owner/repo shorthand)
- [x] **3.4** Implement local adapter (filesystem paths)
- [x] **3.5** Source resolution router in SPEC order
- [x] **3.6** Unit tests for each adapter

---

### Phase 4 — Install and Remove ✓

The core install/remove flow without security scanning.

- [x] **4.1** Implement `core/src/install.ts` (full orchestration: resolve → clone → scan → select → place → record)
- [x] **4.2** Implement `core/src/symlink.ts` (agent-specific symlinks)
- [x] **4.3** Implement `removeSkill()` (symlinks, directory, cache, installed.json)
- [x] **4.4** Integration tests: install standalone, multi-skill, remove, verify state

---

### Phase 5 — Security Scanning (Layer 1 — Static) ✓

Pattern-matching scanner that runs on every install.

- [x] **5.1** Implement 7 detector functions in `core/src/security/patterns.ts`
- [x] **5.2** Implement `scanStatic()` and `scanDiff()` in `core/src/security/static.ts`
- [x] **5.3** Wire scanning into install flow
- [x] **5.4** Build `malicious-skill/` test fixture
- [x] **5.5** Unit tests: every detection category
- [x] **5.6** Integration test: install malicious fixture, verify warnings

---

### Phase 6 — CLI Commands (Core Set) ✓

Wire core logic to CLI commands with interactive UI.

- [x] **6.1** Set up citty command structure (install, remove, list, info, link, unlink)
- [x] **6.2** Implement `cli/src/ui/` (format, prompts, scan display)
- [x] **6.3** `skilltap install` — full interactive flow with all flag combinations
- [x] **6.4** `skilltap remove` — confirm prompt, `--yes`, `--project`
- [x] **6.5** `skilltap list` — global/project grouping, `--json`, empty state
- [x] **6.6** `skilltap info` — installed/available/linked/not-found states
- [x] **6.7** `skilltap link` / `skilltap unlink`
- [x] **6.8** CLI tests

---

### Phase 7 — Tap Management ✓

Add tap support — the curated index model.

- [x] **7.1** Implement `core/src/taps.ts` (add, remove, update, load, search)
- [x] **7.2** Wire tap name resolution into install flow
- [x] **7.3** CLI commands: tap add/remove/list/update/init
- [x] **7.4** `skilltap find` — search across taps, `--json`
- [x] **7.5** `skilltap find -i` — interactive fuzzy finder
- [x] **7.6** `skilltap install <name>` — resolve from taps
- [x] **7.7** `skilltap install <name>@<ref>` — version pinning
- [x] **7.8** Integration tests
- [x] **7.9** Create `sample-tap/tap.json` test fixture

---

### Phase 8 — Update Flow ✓

Diff-aware updates with security re-scanning.

- [x] **8.1** Implement update logic (fetch, compare SHAs, diff, scan diff, apply)
- [x] **8.2** CLI `skilltap update [name]` with diff summary, scan, `--yes`, `--strict`
- [x] **8.3** Integration tests

---

### Phase 9 — Security Scanning (Layer 2 — Semantic) ✓

Agent-based evaluation for deeper analysis.

- [x] **9.1** Implement agent adapters (claude, gemini, codex, opencode, ollama, custom)
- [x] **9.2** JSON extraction pipeline (direct → code block → regex → Zod)
- [x] **9.3** Implement chunking algorithm (paragraph → sentence → hard split + overlap)
- [x] **9.4** Pre-scan chunks for tag injection, escape and auto-flag
- [x] **9.5** Security prompt template with randomized wrapper tags
- [x] **9.6** Parallel chunk evaluation (max 4 concurrent)
- [x] **9.7** Score aggregation, threshold filtering, sorted output
- [x] **9.8** Wire into install/update flows
- [x] **9.9** Unit tests: chunking, JSON extraction, tag injection escaping
- [x] **9.10** Integration test with mock agent

---

### Phase 10 — Config Wizard and Agent Mode ✓

Interactive setup and the agent-safety layer.

- [x] **10.1** `skilltap config` wizard
- [x] **10.2** `skilltap config agent-mode` wizard
- [x] **10.3** Agent mode runtime behavior (plain text, strict, no bypass)
- [x] **10.4** Security policy composition (`composePolicy()`)
- [x] **10.5** Tests: agent mode output, TTY rejection, policy composition matrix

---

### Phase 11 — Polish, Edge Cases, Build

Finalize for v0.1 release.

- [x] **11.1** Error messages and hints for all conditions in SPEC error table
- [x] **11.2** `--json` output for `list`, `find`, `info`
- [x] **11.3** Terminal width handling (truncate descriptions, responsive tables)
- [x] **11.4** Empty state messages for all commands
- [x] **11.5** `bun build --compile` — produce standalone binary
- [x] **11.6** npm publish setup: `skilltap` (cli) and `@skilltap/core` packages
- [ ] **11.7** `bunx skilltap` / `npx skilltap` verification (requires published package)
- [x] **11.8** End-to-end test: fresh config → add tap → find → install → list → update → remove
- [x] **11.9** README with quickstart

**Exit criteria:** `skilltap` is installable via `bunx`, `npx`, or standalone binary. All v0.1 features from SPEC work end-to-end.

---

## v0.2 — Adapters + Ecosystem

### Phase 12 — npm Source Adapter

> Design doc: [DESIGN-NPM-ADAPTER.md](./DESIGN-NPM-ADAPTER.md)

Install skills published as npm packages. Opens access to the 69K+ skills already on npm via skills.sh, vibe-rules, skills-npm, and others.

- [x] **12.1** Implement `core/src/npm-registry.ts` — npm registry API client (fetch metadata, resolve version, search, download + extract tarball)
- [x] **12.2** Implement `core/src/adapters/npm.ts` — `canHandle("npm:")`, resolve to tarball URL, parse `@scope/name@version`
- [x] **12.3** Wire npm adapter into `ADAPTERS[]` array (after github: prefix, before local paths)
- [x] **12.4** Extend scanner to check `skills/*/SKILL.md` as a priority path (antfu/skillpm convention) without deep-scan prompt
- [x] **12.5** Implement tarball integrity verification (SHA-512 SRI hash)
- [x] **12.6** Handle npm-sourced skill updates (version comparison instead of SHA, file-level diff)
- [x] **12.7** Private registry support — read registry URL and auth from `.npmrc` and env vars
- [x] **12.8** Allow `npm:` sources in tap.json `repo` field
- [x] **12.9** Unit tests: adapter canHandle/resolve, version parsing, tarball extraction
- [ ] **12.10** Integration tests: install from npm, update npm-sourced skill
- [ ] **12.11** Test fixture: pre-built npm tarball with known skill structure

**Exit criteria:** `skilltap install npm:@scope/name` works end-to-end. Taps can reference npm sources.

---

### Phase 13 — Community Trust Signals

> Design doc: [DESIGN-TRUST.md](./DESIGN-TRUST.md)

Provenance verification and trust metadata — without managing users. Piggybacks on npm provenance (Sigstore/SLSA) and GitHub attestations.

- [x] **13.1** Define `TrustInfoSchema` in `core/src/trust/types.ts` (four tiers: provenance, publisher, curated, unverified)
- [x] **13.2** Implement `core/src/trust/verify-npm.ts` — fetch npm attestations, verify Sigstore bundle via `sigstore-js`
- [x] **13.3** Implement `core/src/trust/verify-github.ts` — verify GitHub attestations via `gh attestation verify` (optional, when `gh` is on PATH)
- [x] **13.4** Implement `core/src/trust/resolve.ts` — `resolveTrust()` computes tier from available signals
- [x] **13.5** Add optional `trust` field to `InstalledSkillSchema`
- [x] **13.6** Wire trust resolution into install flow (verify after download, store in record)
- [x] **13.7** Re-verify trust on update (new version may have different attestation status)
- [x] **13.8** Add optional `trust` field to `TapSkillSchema` (verified, verifiedBy, verifiedAt)
- [x] **13.9** Display trust tier in `list`, `info`, `find` output
- [x] **13.10** Agent mode: include trust tier in plain text output
- [x] **13.11** Install `sigstore-js` dependency
- [x] **13.12** Unit tests: tier resolution logic, attestation response parsing, display formatting
- [ ] **13.13** Integration tests: install with provenance, install from verified tap

**Exit criteria:** npm-sourced skills show provenance status. Git-sourced skills show GitHub attestation status when available. Trust tier displayed in list/info output. Verification failures degrade gracefully.

---

### Phase 14 — HTTP Registry Adapter

> Design doc: [DESIGN-HTTP-REGISTRY.md](./DESIGN-HTTP-REGISTRY.md)

Support HTTP registries as a tap type — for enterprise, large indexes, and dynamic registries.

- [x] **14.1** Define registry response schemas in `core/src/registry/types.ts` (RegistrySkillSchema, RegistryListResponseSchema, RegistryDetailResponseSchema)
- [x] **14.2** Implement `core/src/registry/client.ts` — HTTP client with auth (bearer token, env var), error handling, response validation
- [x] **14.3** Add `type` and `auth_token`/`auth_env` fields to tap config schema
- [x] **14.4** Implement tap type auto-detection in `tap add` (try JSON, fall back to git clone)
- [x] **14.5** Wire HTTP taps into `loadTaps()` / `searchTaps()` — fetch from API instead of reading local tap.json
- [x] **14.6** Handle `source.type` dispatch in install flow (git, github, npm, url → existing adapters)
- [x] **14.7** Implement direct tarball download for `source.type: "url"` sources
- [x] **14.8** `tap list` shows type column (git/http) and live skill count for HTTP taps
- [x] **14.9** `tap update` is no-op for HTTP taps (always live)
- [x] **14.10** Unit tests: response schema validation, auth header construction, type detection
- [x] **14.11** Integration tests: tap add HTTP (mock Bun.serve), find across git + HTTP taps, install from HTTP registry
- [ ] **14.12** Test fixture: static registry JSON files

**Exit criteria:** `skilltap tap add name https://registry.example.com/skilltap/v1` works. Search and install through HTTP registries works. Auth (bearer token, env var) works. Static file hosting works as a valid registry.

---

### Phase 15 — Distribution

> Design doc: [DESIGN-DISTRIBUTION.md](./DESIGN-DISTRIBUTION.md)

Homebrew formula, install script, GitHub Releases CI.

- [x] **15.1** GitHub Actions release workflow — build 4 binaries (linux-x64, linux-arm64, darwin-x64, darwin-arm64) on tag push
- [x] **15.2** Binary attestation with `actions/attest-build-provenance`
- [x] **15.3** Generate `checksums.txt` (sha256sum) and upload as release asset
- [x] **15.4** npm publish step in release workflow (`--provenance` for both `skilltap` and `@skilltap/core`)
- [x] **15.5** Create `skilltap/homebrew-skilltap` tap repo with `Formula/skilltap.rb` (see `homebrew-skilltap/` — copy to separate repo)
- [x] **15.6** Homebrew formula auto-update workflow (repository_dispatch from main repo → PR to bump formula)
- [x] **15.7** Write `scripts/install.sh` — platform detection, checksum verification, PATH check
- [ ] **15.8** Test: release workflow on a test tag, install script in clean Docker container, `brew install --build-from-source`

**Exit criteria:** Pushing a `v*` tag builds binaries for 4 platforms, publishes to npm with provenance, creates a GitHub Release with checksums, and auto-updates the Homebrew formula. Install script works on Linux and macOS.

---

## v0.3 — Authoring + Polish

### Phase 16 — Create and Verify ✓

> Design doc: [DESIGN-PUBLISH.md](./DESIGN-PUBLISH.md)

Skill authoring tools — scaffold new skills and validate them before sharing.

- [x] **16.1** Implement `core/src/validate.ts` — `validateSkill()` shared validation (SKILL.md exists, frontmatter valid, name matches dir, security self-scan, size check)
- [x] **16.2** Implement templates in `core/src/templates/` — `basic.ts`, `npm.ts`, `multi.ts` (embedded TypeScript functions, not files)
- [x] **16.3** `skilltap create [name]` command — interactive prompts (name, description, template, license), non-interactive with flags
- [x] **16.4** npm template: generate `package.json` with `agent-skill` keyword, `.github/workflows/publish.yml` with `--provenance` and `attest-build-provenance`
- [x] **16.5** Multi template: generate `.agents/skills/` structure with prompted skill names
- [x] **16.6** `skilltap verify [path]` command — run `validateSkill()`, display results (exit 0 = valid, exit 1 = invalid; useful as pre-push hook or CI step)
- [x] **16.7** Print next-steps instructions after create, tap.json snippet after verify
- [x] **16.8** Unit tests: template generation, validateSkill with valid/invalid skills
- [x] **16.9** Integration tests: create + verify roundtrip, verify on invalid skill

**Exit criteria:** `skilltap create` scaffolds valid skills with all three templates. `skilltap verify` validates skills and exits 0/1 for CI use. npm publish is handled externally via the generated GitHub Actions workflow.

---

### Phase 17 — Doctor ✓

> Design doc: [DESIGN-DOCTOR.md](./DESIGN-DOCTOR.md)

Diagnostic command that checks environment, config, and state integrity.

- [x] **17.1** Implement `core/src/doctor.ts` — check functions for git, config, dirs, installed.json, skill integrity, symlinks, taps, agent CLIs, npm
- [x] **17.2** `--fix` support — auto-repair where safe (recreate symlinks, remove orphan records, create missing dirs, re-clone missing taps)
- [x] **17.3** `skilltap doctor` command with streaming output (print each check as it completes)
- [x] **17.4** `--json` output for CI/scripting
- [x] **17.5** Exit code 0 for warnings-only, 1 for failures
- [x] **17.6** Unit tests: each check function with valid/invalid/missing state
- [x] **17.7** Integration tests: healthy env, broken state, --fix repairs, --json output

**Exit criteria:** `skilltap doctor` checks all 9 areas. `--fix` repairs fixable issues. `--json` provides machine-readable output. Exit codes are CI-friendly.

---

### Phase 18 — Shell Completions ✓

> Design doc: [DESIGN-COMPLETIONS.md](./DESIGN-COMPLETIONS.md)

Tab-completion for bash, zsh, and fish.

- [x] **18.1** Implement `--get-completions` hidden endpoint (installed-skills, linked-skills, tap-skills, tap-names)
- [x] **18.2** Implement completion script generators in `cli/src/completions/` (bash, zsh, fish)
- [x] **18.3** `skilltap completions <shell>` command — print script to stdout
- [x] **18.4** `skilltap completions <shell> --install` — write to shell-standard location
- [x] **18.5** Dynamic completions: skill names for remove/update/unlink/info, tap names for tap remove/update
- [x] **18.6** Static completions: commands, subcommands, flags, flag values (--also agents, --template types)
- [x] **18.7** Unit tests: script generation, --get-completions handler
- [x] **18.8** Integration tests: completions command output, --install writes to correct path

**Exit criteria:** Tab-completion works for all commands, flags, and dynamic values in bash, zsh, and fish. `--install` writes to the correct shell-standard location.

---

### Phase 19 — v0.3 Polish ✓

Finalize for v0.3 release.

- [x] **19.1** Update SPEC.md with npm adapter, HTTP registry, trust signals, create, verify, doctor, completions
- [x] **19.2** Update ARCH.md with new modules (trust/, registry/, templates/, doctor, completions)
- [x] **19.3** Update UX.md with new commands (create, verify, doctor, completions)
- [x] **19.4** End-to-end test: create → verify → doctor → completions (`e2e-phase19.test.ts`, 15 tests)
- [x] **19.5** README update with v0.3 features

**Exit criteria:** All docs reflect the current state. End-to-end workflow works across all new features.

---

### Post-v0.3 Additions ✓

Features shipped after the v0.3 release:

- [x] **P1** Custom skill registry system — `[registry]` config section with `enabled` list and `[[registry.sources]]` for custom HTTP registries; built-in skills.sh registry included by default; config wizard updated with "Search public registries?" prompt
- [x] **P2** `skilltap find` improvements — multi-word query support (any token must match), results sorted by install count descending, `--local` flag to skip registry searches, `preSelectedSkill` for auto-selection from skills.sh results
- [x] **P3** `skilltap config get` and `skilltap config set` — non-interactive config read/write; settable key allowlist (preference keys only); blocked keys show hints; agent-friendly (silent on success, exit codes)
- [x] **P4** `skilltap skills` command group — unified skill view showing managed + unmanaged skills across all locations (`.agents/`, `.claude/`, `.cursor/`, etc.); `skills adopt` to bring unmanaged skills under management (move + symlink or track-in-place); `skills move` for global↔project migration; existing `list`/`remove`/`info`/`link`/`unlink` moved under `skills` with silent top-level aliases for backwards compatibility

---

## Dependency Graph

```
v0.1 (complete through Phase 10, Phase 11 in progress)
  │
  ├→ Phase 12 (npm adapter)
  │    └→ Phase 13 (trust signals — needs npm for provenance verification)
  │
  ├→ Phase 14 (HTTP registry — independent of npm adapter)
  │
  ├→ Phase 15 (distribution — independent, can run in parallel)
  │
  ├→ Phase 16 (create + verify — independent, can run anytime after v0.1)
  │
  ├→ Phase 17 (doctor — independent, can run anytime after v0.1)
  │
  ├→ Phase 18 (completions — independent, can run anytime after v0.1)
  │
  └→ Phase 19 (polish — after everything else)
```

Phases 12, 14, 15, 16, 17, and 18 can all be developed in parallel. Phase 13 depends on 12 (npm provenance). npm publishing is handled via the GitHub Actions workflow generated by `skilltap create --template npm`, not by a CLI command.

---

## v1.0 — Plugin Support

### Phase 20 — Plugin Detection and Parsing

Read Claude Code (`.claude-plugin/plugin.json`) and Codex (`.codex-plugin/plugin.json`) plugin formats. Extract the portable subset: skills, MCP server configs, and agent definitions.

- [x] **20.1** Define `PluginManifestSchema` (Zod) in `core/src/schemas/plugin.ts` — unified internal representation covering both Claude Code and Codex formats; component types: `skill`, `mcp`, `agent`
- [x] **20.2** Implement Claude Code plugin.json parser in `core/src/plugin/parse-claude.ts` — read `.claude-plugin/plugin.json`, extract skill paths, `.mcp.json` entries, `agents/*.md` files; resolve relative component paths against plugin root
- [x] **20.3** Implement Codex plugin.json parser in `core/src/plugin/parse-codex.ts` — read `.codex-plugin/plugin.json`, extract skill paths, `.mcp.json` entries
- [x] **20.4** Implement plugin detector in `core/src/plugin/detect.ts` — given a cloned directory, detect plugin.json presence (Claude Code → Codex priority), return parsed manifest or `null`
- [x] **20.5** Implement MCP config reader in `core/src/plugin/mcp.ts` — parse `.mcp.json` files from both formats into a normalized `McpServerConfig[]` (name, command, args, env)
- [x] **20.6** Implement agent definition reader in `core/src/plugin/agents.ts` — parse `agents/*.md` files, extract frontmatter (model, effort, maxTurns, tools, isolation) + body content
- [x] **20.7** Unit tests: parse both plugin formats, detect plugin vs. skill-only repo, MCP normalization, agent parsing, malformed/missing fields

**Exit criteria:** Given a cloned repo, skilltap can detect whether it's a plugin (vs. skill-only), parse the manifest from either format, and produce a normalized list of components (skills, MCP servers, agents) with their paths and configs.

---

### Phase 21 — Plugin Storage and Data Model

Plugin as a first-class record in `plugins.json`, with per-component state tracking.

- [x] **21.1** Define `PluginsJsonSchema` in `core/src/schemas/plugins.ts` — `{ version: 1, plugins: PluginRecord[] }`; each record: name, source (repo URL), ref, sha, scope, installedAt, updatedAt, active, components array
- [x] **21.2** Define `PluginComponentSchema` — `{ type: "skill" | "mcp" | "agent", name: string, active: boolean, config?: object }` — MCP components include their server config, agent components include their frontmatter
- [x] **21.3** Implement `core/src/plugin/state.ts` — `loadPlugins()`, `savePlugins()`, `addPlugin()`, `removePlugin()`, `updatePluginComponent()` (toggle active state)
- [x] **21.4** Storage path: `~/.config/skilltap/plugins.json` (global), `{projectRoot}/.agents/plugins.json` (project)
- [x] **21.5** Unit tests: round-trip save/load, add/remove/toggle, schema validation

**Exit criteria:** Plugin records can be created, stored, loaded, and modified. Each component within a plugin has independent active/inactive state.

---

### Phase 22 — MCP Config Injection

Write MCP server entries directly into each target agent's config file.

- [x] **22.1** Define `McpConfigAdapter` interface in `core/src/plugin/mcp-adapters.ts` — `{ agent: string, configPath(scope): string, read(): McpConfig, write(config): Result, addServer(name, config): Result, removeServer(name): Result }`
- [x] **22.2** Implement Claude Code MCP adapter — reads/writes `mcpServers` in `.claude/settings.json` (project) or `~/.claude/settings.json` (global); backs up before first write
- [x] **22.3** Implement Cursor MCP adapter — reads/writes `.cursor/mcp.json` (project) or `~/.cursor/mcp.json` (global)
- [x] **22.4** Implement Codex MCP adapter — reads/writes `.codex/mcp.json` (project) or `~/.codex/mcp.json` (global)
- [x] **22.5** Implement Gemini and Windsurf MCP adapters (basic — may need research on exact config locations)
- [x] **22.6** Add `skilltap:` namespace prefix to injected MCP server names to avoid collisions (e.g., `skilltap:plugin-name:server-name`)
- [x] **22.7** Backup mechanism: copy agent config to `.bak` before first modification; `skilltap doctor` can detect/restore from backups
- [x] **22.8** Variable substitution: resolve `${CLAUDE_PLUGIN_ROOT}` → plugin install path, `${CLAUDE_PLUGIN_DATA}` → persistent data dir
- [x] **22.9** Unit tests: read/write each format, add/remove servers, backup creation, namespace prefixing
- [x] **22.10** Integration tests: inject MCP into mock agent configs, verify idempotent re-injection, verify removal cleans up

**Exit criteria:** skilltap can inject MCP server configs into Claude Code, Cursor, Codex, Gemini, and Windsurf config files. Injection is namespaced, backed up, and reversible.

---

### Phase 23 — Plugin Install Flow

Wire plugin detection into the existing `skilltap install` command. Auto-detect plugins and install all components.

- [x] **23.1** Extend install flow: after cloning, run plugin detection before skill scanning; if plugin detected, offer "Install as plugin?" (or auto-accept with `--yes`)
- [x] **23.2** Plugin install orchestration in `core/src/plugin/install.ts` — extract skills (use existing install machinery), inject MCP configs (per agent), place agent definitions (Claude Code `.claude/agents/` for now)
- [x] **23.3** Security scan: scan all plugin content (skills + agent .md files) through existing static scanner; MCP configs scanned for suspicious commands/URLs
- [x] **23.4** Agent definition placement: copy `agents/*.md` to `.claude/agents/` (global: `~/.claude/agents/`, project: `.claude/agents/`); Claude Code-only for now, extensible later
- [x] **23.5** Record plugin in `plugins.json` with all components and their initial state (all active)
- [x] **23.6** Skills within a plugin are recorded in `plugins.json` only (not duplicated in `installed.json`) — the plugin owns them
- [x] **23.7** Handle `--also` flag: inject MCP configs into all specified agent platforms; create skill symlinks as usual
- [x] **23.8** Handle scope: `--project` / `--global` determines both skill placement and MCP config injection target
- [x] **23.9** Conflict detection: warn if MCP server names collide with existing entries in agent configs
- [x] **23.10** CLI output: show component summary after install ("Installed plugin X: 3 skills, 2 MCP servers, 1 agent")
- [x] **23.11** Agent mode support: plain text output, auto-accept, strict security
- [x] **23.12** Integration tests: install Claude Code plugin, install Codex plugin, verify all components placed correctly

**Exit criteria:** `skilltap install <plugin-repo>` detects the plugin format, installs skills + MCP servers + agents across target platforms, and records everything in `plugins.json`.

---

### Phase 24 — Plugin Management Commands

`skilltap plugin` subcommand group for listing, inspecting, toggling, and removing plugins.

- [x] **24.1** `skilltap plugin` (alias: `skilltap plugins`) — list installed plugins with component counts and status
- [x] **24.2** `skilltap plugin info <name>` — show plugin details: source, scope, all components with active/inactive status
- [x] **24.3** `skilltap plugin toggle <name>` — interactive component picker (checkboxes for each skill, MCP server, agent); toggling a component enables/disables it:
  - **Skill**: move to/from `.disabled/` (existing disable mechanism)
  - **MCP server**: add/remove entry from agent config files
  - **Agent**: move agent .md to/from a `.disabled/` subdirectory
- [x] **24.4** `skilltap plugin toggle <name> --skills` / `--mcps` / `--agents` — category-level bulk toggle (disable/enable all components of a type)
- [x] **24.5** `skilltap plugin remove <name>` — remove all components (skills, MCP entries, agent definitions), clean up `plugins.json`
- [ ] **24.6** `skilltap plugin update [name]` — update plugin source (git pull / npm check), re-extract components, apply changes (new skills installed, removed skills deleted, MCP configs updated) (deferred)
- [x] **24.7** `--json` output for all plugin subcommands
- [x] **24.8** Shell completions: add `plugin` subcommand, plugin name completions for info/toggle/remove
- [x] **24.9** Doctor integration: add plugin checks (plugins.json valid, plugin components exist on disk, MCP entries present in agent configs)
- [x] **24.10** Unit tests: toggle logic, remove cleanup, update diff
- [x] **24.11** Integration tests: full lifecycle (install → list → toggle → info → update → remove)

**Exit criteria:** Plugins can be listed, inspected, toggled at the component level, updated, and removed. All operations are reversible and reflected in both `plugins.json` and agent config files.

---

### Phase 25 — Plugin Polish

- [x] **25.1** Marketplace tap adapter update: `adaptMarketplaceToTap()` now includes a `plugin: true` flag on entries that have MCP/agent components (not just skills), so `skilltap find` can show "plugin" vs "skill" in results
- [x] **25.2** `skilltap find` shows plugin badge for tap entries that are plugins
- [x] **25.3** `skilltap status --json` includes plugin count
- [x] **25.4** Update SPEC.md, ARCH.md, UX.md with final plugin specifications
- [x] **25.5** End-to-end test: install plugin from tap → toggle components → update → remove
- [x] **25.6** README update with plugin features

**Exit criteria:** Plugin support is fully documented, tested end-to-end, and integrated with the existing tap/find/status ecosystem.

---

### Post-Phase 25 Additions ✓

Features shipped after the Phase 25 release:

- [x] **PP1** Tap-defined plugins — `tap.json` now supports a `plugins` array for inline plugin definitions (`TapPluginSchema` with skills, mcpServers, agents); `skilltap install tap-name/plugin-name` resolves tap plugins directly; `tapPluginToManifest()` converts tap entries to `PluginManifest` for `installPlugin()`; `loadTaps()` includes plugin entries alongside skill entries
- [x] **PP2** Marketplace auto-detection — `adaptMarketplaceToTap()` is now async and accepts optional `tapDir`; for marketplace plugins with relative-path sources, auto-detects `.claude-plugin/plugin.json` via `detectPlugin()` and produces `TapPlugin` entries (with full skills/MCP/agents) when found, falls back to `TapSkill` otherwise
- [x] **PP3** `"skilltap"` format — added to `PLUGIN_FORMATS` in `schemas/plugin.ts`; tap-defined plugins use this format value in their `PluginRecord`
- [x] **PP4** Shared helpers — `scopeBase()` in `paths.ts` (replaces inline ternaries); `mcpServerToStored()` in `plugin/state.ts`; `loadJsonState()`/`saveJsonState()` in `json-state.ts`; `AGENT_DEF_PATHS` + `agentDefPath()`/`agentDefDisabledPath()` in `paths.ts`/`symlink.ts`; `SKILLTAP_MCP_PREFIX` constant in `mcp-inject.ts`; `discoverSkills()` in `plugin/parse-common.ts`; `componentSummary()` in `cli/src/ui/plugin-format.ts`
- [x] **PP5** Test infrastructure — `createTestEnv()` and `pathExists()` in `@skilltap/test-utils`; `createTapWithPlugins()` fixture factory

---

## Dependency Graph (updated)

```
v0.1–v0.3 (complete)
  │
  ├→ Phase 20 (plugin detection + parsing)
  │    └→ Phase 21 (plugin storage + data model)
  │         └→ Phase 22 (MCP config injection)
  │              └→ Phase 23 (plugin install flow — needs 20, 21, 22)
  │                   └→ Phase 24 (plugin management commands — needs 23)
  │                        └→ Phase 25 (polish — needs 24)
  │
  └→ Deferred (independent of plugin work)
```

Phases 20–22 can be developed somewhat in parallel (parsing, storage, and MCP injection are mostly independent), but the install flow (23) needs all three, and management (24) needs the install flow.

---

## v2.0 — Tooling-Surface Redesign

This is the major refactor that introduces the project manifest, drops the HTTP registry, retires "agent mode" as a concept, simplifies security config, and adds Cargo-style sync. See [VISION.md — v2.0](./VISION.md#v20-direction-simplification-unification-project-manifest), [SPEC.md — v2.0](./SPEC.md#v20--tooling-surface-redesign), and [ARCH.md — v2.0](./ARCH.md#v20-architecture-additions) for the design.

The phases are ordered for dependency. 26–28 are the data-layer foundation; 29 is the headline behavior (sync); 30–35 are user-facing additions; 36–38 are polish and release. Several phases can run in parallel — see the dependency graph below.

### Phase 26 — v2.0 Schema Foundation

Establish all new Zod schemas before touching behavior. No user-facing change.

- [ ] **26.1** Define `ProjectManifestSchema`, `ManifestEntrySchema`, `TargetsSchema`, `LockfileSchema`, `LockEntrySchema` in `core/src/manifest/schemas.ts`
- [ ] **26.2** Define `PluginManifestV2Schema` in `core/src/plugin-v2/schema.ts` (native `.skilltap/<name>.toml` format)
- [ ] **26.3** Define `ConfigV2Schema`, `SecurityConfigV2Schema`, `AgentConfigSchema` in `core/src/schemas/config-v2.ts`. Keep v1.0 schemas in `core/src/schemas/v1/` for migration.
- [ ] **26.4** Define `StateSchema` (version 2) in `core/src/state/schema.ts` with `skills`, `plugins`, `mcpServers` arrays
- [ ] **26.5** Add range parser/matcher in `core/src/manifest/range.ts` — handle `^`, `~`, `*`, exact tags, branch refs
- [ ] **26.6** Unit tests: every schema with valid + invalid fixtures, range matching across patterns

**Exit criteria:** All v2.0 schemas parse, validate, and have tests. v1.0 schemas remain untouched and continue to work.

---

### Phase 27 — State Consolidation + Migration

Merge `installed.json` + `plugins.json` into `state.json`. Implement `skilltap migrate`.

- [ ] **27.1** Implement `core/src/state/load.ts`, `save.ts` for the unified `state.json` file
- [ ] **27.2** Implement v1.0 detection: presence of `installed.json` or `plugins.json` or any v1.0-only config key returns "v1.0 setup detected"
- [ ] **27.3** Implement `core/src/state/migrate-v1.ts` — read v1.0 files, translate to v2.0 state, write atomically
- [ ] **27.4** Implement config migration (v1.0 `[security.human]` / `[security.agent]` / `[[security.overrides]]` / `[agent-mode]` → v2.0 `[security]` / `[agent]`)
- [ ] **27.5** HTTP-tap handling in migration: list affected taps, error with hint to convert to git or remove. Don't silently drop.
- [ ] **27.6** `skilltap migrate` CLI command: detect, translate, write, run doctor verify, print diff summary
- [ ] **27.7** v2.0 startup detection — `cli/src/index.ts` checks for v1.0 markers; if found, error with hint and exit
- [ ] **27.8** Unit tests: migration of every v1.0 schema permutation, hint output, idempotent re-runs (already-migrated detection)
- [ ] **27.9** Integration test: install via v1.0 binary, run v2.0 migrate, verify state matches expected

**Exit criteria:** v1.0 users can run `skilltap migrate` and get a working v2.0 setup. v2.0 refuses to operate on un-migrated state with a clear hint.

---

### Phase 28 — Project Manifest + Lockfile

Load, save, and resolve the project manifest. No behavior wired in yet.

- [ ] **28.1** Implement `manifest/load.ts` and `save.ts` (with `findProjectRoot()` integration)
- [ ] **28.2** Implement `manifest/resolve.ts` — resolve manifest entries to `ResolvedDeps[]` with source adapter dispatch
- [ ] **28.3** Implement lockfile read/write/atomic-update. Lockfile entries keyed by source string.
- [ ] **28.4** Implement `manifest/publish.ts` — `discoverPublishablePlugins(repoRoot)` returns all `.skilltap/<name>.toml` with `publish = true`
- [ ] **28.5** Unit tests: round-trip manifest, round-trip lockfile, range resolution against fixture sources
- [ ] **28.6** Integration test: write manifest, write lockfile, reload, verify

**Exit criteria:** Manifest and lockfile can be loaded, edited programmatically, and saved. Publishable plugins are discoverable in a repo.

---

### Phase 29 — Sync Engine + Command

The headline v2.0 capability.

- [ ] **29.1** Implement `sync/drift.ts` — given manifest, lockfile, state, compute `DriftReport` (adds, removes, ref-changes, lockfile-only entries)
- [ ] **29.2** Implement `sync/plan.ts` — `planSync()` produces a `SyncPlan` with action list and rationale per item
- [ ] **29.3** Implement `sync/apply.ts` — execute plan via existing install/remove/update machinery; update lockfile after each step
- [ ] **29.4** `skilltap sync` CLI command with `--strict`, `--yes`, `--prune` flags and interactive diff display via @clack/prompts
- [ ] **29.5** Unit tests: plan generation across drift permutations, lockfile-only entry handling
- [ ] **29.6** Integration tests: full sync flow with fixtures, `--strict` exits non-zero on drift, `--prune` removes undeclared, `--yes` auto-applies

**Exit criteria:** `skilltap sync` reconciles manifest/lockfile/state. Teams can commit `skilltap.toml` + `skilltap.lock` and reach parity on a fresh clone.

---

### Phase 30 — Native Plugin Format + Multi-Plugin Repos

Read `.skilltap/<plugin>.toml`. Support multiple plugins per repo.

- [ ] **30.1** Implement `plugin-v2/parse-toml.ts` — TOML reader for native v2.0 plugin format
- [ ] **30.2** Implement `plugin-v2/discover.ts` — find all `.skilltap/*.toml` in a repo, filter by `publish = true`
- [ ] **30.3** Implement `plugin-v2/normalize.ts` — produce existing internal `PluginManifest` shape from `PluginManifestV2`
- [ ] **30.4** Wire native format into `detect.ts` priority order: `.skilltap/` (preferred) → `.claude-plugin/` → `.codex-plugin/`
- [ ] **30.5** Multi-plugin install: parse `user/repo:plugin-name` syntax in `install.ts`. Bare `user/repo` prompts; `--agent` errors with list.
- [ ] **30.6** Validate `publish = true` enforcement — repos without it can't be installed as plugins from outside (still installable as consumer-only repos with `[skills]` / `[plugins]` deps)
- [ ] **30.7** Unit tests: parse v2.0 plugin TOML, multi-plugin discovery, `:name` syntax parsing
- [ ] **30.8** Integration test: install single plugin from multi-plugin repo, install all with `:*`, error in agent mode with multiple

**Exit criteria:** Repos can publish multiple plugins via `.skilltap/<name>.toml`. Users select which to install. Backwards compat: existing `.claude-plugin/` / `.codex-plugin/` formats keep working.

---

### Phase 31 — Security Simplification

Collapse the v1.0 security model. Remove HTTP registry adapter.

- [ ] **31.1** Rewrite `policy/compose.ts` — single rule, no human/agent split, trust-list short-circuit
- [ ] **31.2** Move semantic-scan opt-in: only `scan = "semantic"` in config OR `--deep` flag on the call enables it. Default config never enables it.
- [ ] **31.3** Implement glob matcher for `trust = []` (matches against tap name OR full source URL)
- [ ] **31.4** Remove HTTP registry tap adapter (`core/src/registry/`, registry-related types in tap config schema)
- [ ] **31.5** Remove security presets (`PRESET_VALUES`, `SECURITY_PRESETS`)
- [ ] **31.6** Remove `[[security.overrides]]` parsing (kept in v1.0 schemas for migration only)
- [ ] **31.7** Update `installSkill` / `installPlugin` to use the new policy
- [ ] **31.8** Update `policy.ts` UI helpers to render new policy explanation strings
- [ ] **31.9** Unit tests: trust-list matching, on_warn = install proceeds without prompt, scan = none skips entirely
- [ ] **31.10** Update existing security tests to v2.0 policy (delete or rewrite v1.0 mode-split assertions)

**Exit criteria:** Security config is one block with three keys. Trust list short-circuits scanning. HTTP registry adapter gone. All v1.0 security tests either deleted, migrated, or kept for v1.0 schema compatibility tests.

---

### Phase 32 — Agent Flag

Replace agent-mode code paths.

- [ ] **32.1** Implement `agent-flag/resolve.ts` — combine `--agent` flag, `SKILLTAP_AGENT` env, `[agent] default` config into single boolean
- [ ] **32.2** Implement `agent-flag/enforce.ts` — return error if `[agent] block = true` and `--agent` requested
- [ ] **32.3** Replace `config["agent-mode"].enabled` checks across the codebase with `effective.agent`
- [ ] **32.4** Remove `[agent-mode]` from v2.0 config schema (kept in v1.0 for migration)
- [ ] **32.5** Remove `skilltap config agent-mode` interactive command. Add `skilltap config set agent.default true|false` and `agent.block`.
- [ ] **32.6** Update `--agent` behavior: no prompts, plain text, auto-pick when single option, error when ambiguous
- [ ] **32.7** Verify security policy is unchanged when `--agent` is set (no special agent-mode rules)
- [ ] **32.8** Unit tests: flag resolution permutations (flag + env + config), block enforcement
- [ ] **32.9** Integration tests: `--agent` + sync (auto-applies if --yes equivalent), `--agent` + multi-plugin (errors with hint)

**Exit criteria:** `--agent` flag is the only mechanism. Agent-mode block + scope removed. Security treats `--agent` and human equally.

---

### Phase 33 — Smart Scope + Status Dashboard

Two interlocking DX wins.

- [ ] **33.1** Implement smart scope default in `policy/compose.ts` — `findProjectRoot()` → project; otherwise global. Always include resolved scope in install output.
- [ ] **33.2** Implement `cli/src/commands/status.ts` — gather state (skills, plugins, MCP injection per agent, taps, updates, drift), render text dashboard
- [ ] **33.3** Wire bare `skilltap` (no args) to status command (was citty default `--help`)
- [ ] **33.4** `--json` output for status
- [ ] **33.5** Drift line in status: "manifest declares N items not installed. Run `skilltap sync`." Updates line: "N updates available. Run `skilltap update`."
- [ ] **33.6** Unit tests: status snapshot rendering, --json schema
- [ ] **33.7** Integration tests: clean state, drift state, --json output

**Exit criteria:** `skilltap` opens a status dashboard. Smart scope removes the default-scope prompt in git repos.

---

### Phase 34 — Component-Ref Syntax + Toggle Promotion

Top-level toggle/enable/disable with `:component` syntax.

- [ ] **34.1** Implement `name:component` parser in shared util
- [ ] **34.2** Top-level `skilltap toggle <name>[:component]`, `skilltap enable <name>[:component]`, `skilltap disable <name>[:component]`
- [ ] **34.3** Bare `name` opens picker (existing behavior); `name:component` direct toggle
- [ ] **34.4** Update completions to suggest `name:component` after `:`
- [ ] **34.5** Keep existing `skilltap plugin toggle` etc. as silent aliases
- [ ] **34.6** Unit tests: parser edge cases (multiple colons, missing name, missing component)
- [ ] **34.7** Integration tests: direct toggle, picker fallback

**Exit criteria:** Users can address components directly without going through a picker.

---

### Phase 35 — Try + MCP-Only Install + Claude Desktop

Three smaller v2.0 additions bundled.

- [ ] **35.1** Implement `core/src/try.ts` — clone to temp, parse manifests, run scan, render summary, cleanup
- [ ] **35.2** `skilltap try <source>` CLI command
- [ ] **35.3** `mcp:` source prefix in `install.ts` — extract `[[servers]]` only, inject into agent configs, track in `state.json` `mcpServers` array
- [ ] **35.4** `skilltap remove mcp:<name>` for symmetric removal
- [ ] **35.5** Add `claude-desktop` to `MCP_AGENT_CONFIGS` registry with platform-specific paths (macOS, Windows, Linux)
- [ ] **35.6** Unit tests: try cleanup behavior, mcp-only install/remove, claude-desktop config path resolution per platform
- [ ] **35.7** Integration tests: `skilltap try` against fixture repo (no install side-effect), `skilltap install mcp:` round-trip, claude-desktop injection

**Exit criteria:** `try` previews safely. `mcp:` installs servers without skill machinery. Claude Desktop is supported.

---

### Phase 36 — Doctor v2.0 Upgrades

Drift and consistency checks.

- [ ] **36.1** Add manifest-vs-state drift check
- [ ] **36.2** Add lockfile-vs-state drift check
- [ ] **36.3** Add `.skilltap/<name>.toml` validity check (parse + required fields)
- [ ] **36.4** Add MCP injection consistency check (state ↔ agent config files, both directions)
- [ ] **36.5** Extend `--fix`: prune state-orphan MCP entries from agent configs, regenerate missing lockfile entries from state
- [ ] **36.6** Unit tests for each new check
- [ ] **36.7** Integration tests: synthetic drift scenarios, --fix repairs

**Exit criteria:** Doctor catches manifest/lockfile/state drift and MCP inconsistencies. `--fix` repairs the safely-fixable subset.

---

### Phase 37 — Command Surface Promotion + Aliases

Top-level shortcuts and back-compat aliases.

- [ ] **37.1** Top-level commands: `sync`, `status`, `try`, `migrate`, `enable`, `disable` (added in earlier phases — confirm wiring)
- [ ] **37.2** Top-level `toggle` (already added in Phase 34) — confirm alias from `skilltap plugin toggle`
- [ ] **37.3** Silent aliases for v1.0 paths: `skilltap remove` → `skilltap skills remove` (or top-level), `skilltap list` → `skilltap list` (already top-level), `skilltap plugins` → `skilltap plugin`
- [ ] **37.4** Update bash/zsh/fish completion scripts for all new commands and `:component` dynamic completions
- [ ] **37.5** Verify no breaking change in existing v1.0 command paths
- [ ] **37.6** Integration tests: every v1.0 command path still works (silent alias verification)

**Exit criteria:** v2.0 commands feel flat for daily use. v1.0 paths still work for users with muscle memory.

---

### Phase 38 — v2.0 Polish + Docs + Release

- [ ] **38.1** Update README with v2.0 quickstart (manifest, sync, simplified config)
- [ ] **38.2** Update website (`website/`) with new commands, status dashboard screenshots, manifest examples
- [ ] **38.3** Update `llms-full.txt` for LLM ingestion
- [ ] **38.4** Update CLAUDE.md / AGENTS.md with v2.0 conventions
- [ ] **38.5** End-to-end test: clean v2.0 init → install → manifest write → sync on fresh clone → toggle → migrate from v1.0 → status dashboard
- [ ] **38.6** CHANGELOG entry for v2.0 with migration guide
- [ ] **38.7** Bump version to 2.0.0
- [ ] **38.8** Release workflow verification (binaries, npm publish, Homebrew formula update)

**Exit criteria:** v2.0 ships. Docs reflect v2.0. v1.0 users have a clear migration path.

---

### v2.0 Dependency Graph

```
Phase 26 (schemas) ────┬─→ Phase 27 (state + migrate) ─→ Phase 32 (agent flag)
                        │                                     │
                        ├─→ Phase 28 (manifest+lock) ─────────┤
                        │         │                           │
                        │         └─→ Phase 29 (sync) ───┐    │
                        │                                │    │
                        ├─→ Phase 30 (native plugin) ────┤    │
                        │                                │    │
                        └─→ Phase 31 (security simpl.) ──┤    │
                                                         │    │
                                                         ▼    ▼
                                            Phase 33 (smart scope + status)
                                            Phase 34 (component-ref toggle)
                                            Phase 35 (try + mcp + desktop)
                                            Phase 36 (doctor v2)
                                                         │
                                                         ▼
                                            Phase 37 (surface promotion)
                                                         │
                                                         ▼
                                            Phase 38 (polish + release)
```

Phases 28, 30, 31, 32 can run mostly in parallel after 27. Phases 33–36 can run in parallel after their dependencies are met. 37 needs 33–36; 38 is the final integration.

---

## What's Deferred to v1.1+

- Windows support
- Linux distro packages (.deb, .rpm, AUR, Nix)
- `security.require_provenance` config option (block unverified skills)
- Direct LLM API integrations for semantic scan (Anthropic API, OpenAI API — bypassing CLI)
- `skilltap tap export --format http` (generate static HTTP registry from tap.json)
- Plugin for popular editors (VS Code extension)
- Skill dependency system
- SBOM generation for installed skills
- Plugin hooks support (Claude Code hooks.json — platform-specific, lower priority)
- Plugin LSP server support (Claude Code .lsp.json)
- Plugin commands support (Claude Code commands/*.md)
- Agent definitions for non-Claude-Code platforms (when other agents adopt the format)
- Plugin user config / secrets management (Claude Code userConfig with keychain)
