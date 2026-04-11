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
