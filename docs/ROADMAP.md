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

- [ ] **11.1** Error messages and hints for all conditions in SPEC error table
- [ ] **11.2** `--json` output for `list`, `find`, `info`
- [ ] **11.3** Terminal width handling (truncate descriptions, responsive tables)
- [ ] **11.4** Empty state messages for all commands
- [x] **11.5** `bun build --compile` — produce standalone binary
- [ ] **11.6** npm publish setup: `skilltap` (cli) and `@skilltap/core` packages
- [ ] **11.7** `bunx skilltap` / `npx skilltap` verification
- [ ] **11.8** End-to-end test: fresh config → add tap → find → install → list → update → remove
- [ ] **11.9** README with quickstart

**Exit criteria:** `skilltap` is installable via `bunx`, `npx`, or standalone binary. All v0.1 features from SPEC work end-to-end.

---

## v0.2 — Adapters + Ecosystem

### Phase 12 — npm Source Adapter

> Design doc: [DESIGN-NPM-ADAPTER.md](./DESIGN-NPM-ADAPTER.md)

Install skills published as npm packages. Opens access to the 69K+ skills already on npm via skills.sh, vibe-rules, skills-npm, and others.

- [ ] **12.1** Implement `core/src/npm-registry.ts` — npm registry API client (fetch metadata, resolve version, search, download + extract tarball)
- [ ] **12.2** Implement `core/src/adapters/npm.ts` — `canHandle("npm:")`, resolve to tarball URL, parse `@scope/name@version`
- [ ] **12.3** Wire npm adapter into `ADAPTERS[]` array (after github: prefix, before local paths)
- [ ] **12.4** Extend scanner to check `skills/*/SKILL.md` as a priority path (antfu/skillpm convention) without deep-scan prompt
- [ ] **12.5** Implement tarball integrity verification (SHA-512 SRI hash)
- [ ] **12.6** Handle npm-sourced skill updates (version comparison instead of SHA, file-level diff)
- [ ] **12.7** Private registry support — read registry URL and auth from `.npmrc` and env vars
- [ ] **12.8** `skilltap find --npm <query>` — search npm registry API with `keywords:agent-skill` filter
- [ ] **12.9** Allow `npm:` sources in tap.json `repo` field
- [ ] **12.10** Unit tests: adapter canHandle/resolve, version parsing, tarball extraction
- [ ] **12.11** Integration tests: install from npm, find --npm, update npm-sourced skill
- [ ] **12.12** Test fixture: pre-built npm tarball with known skill structure

**Exit criteria:** `skilltap install npm:@scope/name` works end-to-end. `skilltap find --npm` searches the npm registry. Taps can reference npm sources.

---

### Phase 13 — Community Trust Signals

> Design doc: [DESIGN-TRUST.md](./DESIGN-TRUST.md)

Provenance verification and trust metadata — without managing users. Piggybacks on npm provenance (Sigstore/SLSA) and GitHub attestations.

- [ ] **13.1** Define `TrustInfoSchema` in `core/src/trust/types.ts` (four tiers: provenance, publisher, curated, unverified)
- [ ] **13.2** Implement `core/src/trust/verify-npm.ts` — fetch npm attestations, verify Sigstore bundle via `sigstore-js`
- [ ] **13.3** Implement `core/src/trust/verify-github.ts` — verify GitHub attestations via `gh attestation verify` (optional, when `gh` is on PATH)
- [ ] **13.4** Implement `core/src/trust/resolve.ts` — `resolveTrust()` computes tier from available signals
- [ ] **13.5** Add optional `trust` field to `InstalledSkillSchema`
- [ ] **13.6** Wire trust resolution into install flow (verify after download, store in record)
- [ ] **13.7** Re-verify trust on update (new version may have different attestation status)
- [ ] **13.8** Add optional `trust` field to `TapSkillSchema` (verified, verifiedBy, verifiedAt)
- [ ] **13.9** Display trust tier in `list`, `info`, `find` output
- [ ] **13.10** Agent mode: include trust tier in plain text output
- [ ] **13.11** Install `sigstore-js` dependency
- [ ] **13.12** Unit tests: tier resolution logic, attestation response parsing, display formatting
- [ ] **13.13** Integration tests: install with provenance, install from verified tap

**Exit criteria:** npm-sourced skills show provenance status. Git-sourced skills show GitHub attestation status when available. Trust tier displayed in list/info output. Verification failures degrade gracefully.

---

### Phase 14 — HTTP Registry Adapter

> Design doc: [DESIGN-HTTP-REGISTRY.md](./DESIGN-HTTP-REGISTRY.md)

Support HTTP registries as a tap type — for enterprise, large indexes, and dynamic registries.

- [ ] **14.1** Define registry response schemas in `core/src/registry/types.ts` (RegistrySkillSchema, RegistryListResponseSchema, RegistryDetailResponseSchema)
- [ ] **14.2** Implement `core/src/registry/client.ts` — HTTP client with auth (bearer token, env var), error handling, response validation
- [ ] **14.3** Add `type` and `auth_token`/`auth_env` fields to tap config schema
- [ ] **14.4** Implement tap type auto-detection in `tap add` (try JSON, fall back to git clone)
- [ ] **14.5** Wire HTTP taps into `loadTaps()` / `searchTaps()` — fetch from API instead of reading local tap.json
- [ ] **14.6** Handle `source.type` dispatch in install flow (git, github, npm, url → existing adapters)
- [ ] **14.7** Implement direct tarball download for `source.type: "url"` sources
- [ ] **14.8** `tap list` shows type column (git/http) and live skill count for HTTP taps
- [ ] **14.9** `tap update` is no-op for HTTP taps (always live)
- [ ] **14.10** Unit tests: response schema validation, auth header construction, type detection
- [ ] **14.11** Integration tests: tap add HTTP (mock Bun.serve), find across git + HTTP taps, install from HTTP registry
- [ ] **14.12** Test fixture: static registry JSON files

**Exit criteria:** `skilltap tap add name https://registry.example.com/skilltap/v1` works. Search and install through HTTP registries works. Auth (bearer token, env var) works. Static file hosting works as a valid registry.

---

### Phase 15 — Distribution

> Design doc: [DESIGN-DISTRIBUTION.md](./DESIGN-DISTRIBUTION.md)

Homebrew formula, install script, GitHub Releases CI.

- [ ] **15.1** GitHub Actions release workflow — build 4 binaries (linux-x64, linux-arm64, darwin-x64, darwin-arm64) on tag push
- [ ] **15.2** Binary attestation with `actions/attest-build-provenance`
- [ ] **15.3** Generate `checksums.txt` (sha256sum) and upload as release asset
- [ ] **15.4** npm publish step in release workflow (`--provenance` for both `skilltap` and `@skilltap/core`)
- [ ] **15.5** Create `skilltap/homebrew-skilltap` tap repo with `Formula/skilltap.rb`
- [ ] **15.6** Homebrew formula auto-update workflow (repository_dispatch from main repo → PR to bump formula)
- [ ] **15.7** Write `scripts/install.sh` — platform detection, checksum verification, PATH check
- [ ] **15.8** Test: release workflow on a test tag, install script in clean Docker container, `brew install --build-from-source`

**Exit criteria:** Pushing a `v*` tag builds binaries for 4 platforms, publishes to npm with provenance, creates a GitHub Release with checksums, and auto-updates the Homebrew formula. Install script works on Linux and macOS.

---

## v0.3 — Authoring + Polish

### Phase 16 — Publish and Create

> Design doc: [DESIGN-PUBLISH.md](./DESIGN-PUBLISH.md)

Skill authoring tools — scaffold new skills and publish to npm or validate for git.

- [ ] **16.1** Implement `core/src/validate.ts` — `validateSkill()` shared validation (SKILL.md exists, frontmatter valid, name matches dir, security self-scan, size check)
- [ ] **16.2** Implement templates in `core/src/templates/` — `basic.ts`, `npm.ts`, `multi.ts` (embedded TypeScript functions, not files)
- [ ] **16.3** `skilltap create [name]` command — interactive prompts (name, description, template, license), non-interactive with flags
- [ ] **16.4** npm template: generate `package.json` with `agent-skill` keyword, `.github/workflows/publish.yml` with `--provenance` and `attest-build-provenance`
- [ ] **16.5** Multi template: generate `.agents/skills/` structure with prompted skill names
- [ ] **16.6** `skilltap publish [path]` command — run `validateSkill()`, display results
- [ ] **16.7** `skilltap publish --npm` — check package.json, verify npm auth, run `npm publish --provenance`
- [ ] **16.8** `skilltap publish --dry-run` — validate only, no publish (CI/pre-commit hook use case)
- [ ] **16.9** Print next-steps instructions after create, tap.json snippet after publish
- [ ] **16.10** Unit tests: template generation, validateSkill with valid/invalid skills
- [ ] **16.11** Integration tests: create + link roundtrip, publish --dry-run, publish --npm --dry-run

**Exit criteria:** `skilltap create` scaffolds valid skills with all three templates. `skilltap publish --dry-run` validates skills. `skilltap publish --npm` publishes to npm with provenance.

---

### Phase 17 — Doctor

> Design doc: [DESIGN-DOCTOR.md](./DESIGN-DOCTOR.md)

Diagnostic command that checks environment, config, and state integrity.

- [ ] **17.1** Implement `core/src/doctor.ts` — check functions for git, config, dirs, installed.json, skill integrity, symlinks, taps, agent CLIs, npm
- [ ] **17.2** `--fix` support — auto-repair where safe (recreate symlinks, remove orphan records, create missing dirs, re-clone missing taps)
- [ ] **17.3** `skilltap doctor` command with streaming output (print each check as it completes)
- [ ] **17.4** `--json` output for CI/scripting
- [ ] **17.5** Exit code 0 for warnings-only, 1 for failures
- [ ] **17.6** Unit tests: each check function with valid/invalid/missing state
- [ ] **17.7** Integration tests: healthy env, broken state, --fix repairs, --json output

**Exit criteria:** `skilltap doctor` checks all 9 areas. `--fix` repairs fixable issues. `--json` provides machine-readable output. Exit codes are CI-friendly.

---

### Phase 18 — Shell Completions

> Design doc: [DESIGN-COMPLETIONS.md](./DESIGN-COMPLETIONS.md)

Tab-completion for bash, zsh, and fish.

- [ ] **18.1** Implement `--get-completions` hidden endpoint (installed-skills, linked-skills, tap-skills, tap-names)
- [ ] **18.2** Implement completion script generators in `cli/src/completions/` (bash, zsh, fish)
- [ ] **18.3** `skilltap completions <shell>` command — print script to stdout
- [ ] **18.4** `skilltap completions <shell> --install` — write to shell-standard location
- [ ] **18.5** Dynamic completions: skill names for remove/update/unlink/info, tap names for tap remove/update
- [ ] **18.6** Static completions: commands, subcommands, flags, flag values (--also agents, --template types)
- [ ] **18.7** Unit tests: script generation, --get-completions handler
- [ ] **18.8** Integration tests: completions command output, --install writes to correct path

**Exit criteria:** Tab-completion works for all commands, flags, and dynamic values in bash, zsh, and fish. `--install` writes to the correct shell-standard location.

---

### Phase 19 — v0.3 Polish

Finalize for v0.3 release.

- [ ] **19.1** Update SPEC.md with npm adapter, HTTP registry, trust signals, create, publish, doctor, completions
- [ ] **19.2** Update ARCH.md with new modules (trust/, registry/, templates/, doctor, completions)
- [ ] **19.3** Update UX.md with new commands (create, publish, doctor, completions, find --npm)
- [ ] **19.4** End-to-end test: create → publish --dry-run → install npm: → doctor → completions
- [ ] **19.5** README update with v0.3 features

**Exit criteria:** All docs reflect the current state. End-to-end workflow works across all new features.

---

## Dependency Graph

```
v0.1 (complete through Phase 10, Phase 11 in progress)
  │
  ├→ Phase 12 (npm adapter)
  │    └→ Phase 13 (trust signals — needs npm for provenance verification)
  │         └→ Phase 16 (publish — needs trust for --provenance workflow)
  │
  ├→ Phase 14 (HTTP registry — independent of npm adapter)
  │
  ├→ Phase 15 (distribution — independent, can run in parallel)
  │
  ├→ Phase 17 (doctor — independent, can run anytime after v0.1)
  │
  ├→ Phase 18 (completions — independent, can run anytime after v0.1)
  │
  └→ Phase 16 (create — independent of npm, but publish --npm needs Phase 12)
       └→ Phase 19 (polish — after everything else)
```

Phases 12, 14, 15, 17, and 18 can all be developed in parallel. Phase 13 depends on 12 (npm provenance). Phase 16's `publish --npm` depends on 12 (npm adapter), but `create` and `publish --dry-run` are independent.

---

## What's Deferred to v0.4+

- Windows support
- Linux distro packages (.deb, .rpm, AUR, Nix)
- `security.require_provenance` config option (block unverified skills)
- Direct LLM API integrations for semantic scan (Anthropic API, OpenAI API — bypassing CLI)
- `skilltap tap export --format http` (generate static HTTP registry from tap.json)
- Plugin for popular editors (VS Code extension)
- Skill dependency system
- SBOM generation for installed skills
