# E2E Test Suite Design: v2.0/v2.1 Release

## Project Summary

skilltap is a CLI tool that installs agent skills (SKILL.md) and plugins from any
git host. The v2.0/v2.1 release introduced a substantial new surface:

- **`state.json`** — single canonical store replacing v0.x `installed.json` + `plugins.json`
- **`skilltap.toml` + `skilltap.lock`** — Cargo-style project manifest + lockfile
- **`skilltap sync`** — read-only drift report + `--apply` reconciliation
- **`skilltap migrate`** — one-shot v1 → v2 upgrade
- **`skilltap try`** — read-only preview (the never-writes safety surface)
- **Smart-scope-default** — infer project (in git repo) or global (outside)
- **Agent-mode entry points** — `--agent` flag, `SKILLTAP_AGENT=1` env, `[agent-mode]` config
- **HTTP registry tap removal** — Phase 31b (silent filter + warning)
- **MCP-only install** — `mcp:<source>` syntax, separate state.json bucket
- **v2.0 doctor checks** — state-v2, manifest-drift, lockfile-drift, mcp-consistency, plugin-manifests, v1-orphans

This doc enumerates the tests that prove the v2.0 release does what it promises,
end-to-end. Existing unit and subprocess tests cover most of the schema and core-
function surface; this design focuses on integration journeys and failure modes
that span multiple modules.

## Test Environment

- **Framework**: `bun:test` (`describe`, `test`, `expect`)
- **Subprocess driver**: `runSkilltap(args, homeDir, configDir, cwd?)` from `@skilltap/test-utils` — pipe-mode (non-TTY)
- **PTY driver**: `runInteractive(args, { cwd, env })` for clack-rendered flows
- **Fixtures**: `createStandaloneSkillRepo`, `createMultiSkillRepo`, `createTestEnv`, `makeTmpDir`, `initRepo`, `commitAll`
- **Env isolation**: `SKILLTAP_HOME` + `XDG_CONFIG_HOME` per test (`createTestEnv()` returns both)
- **Cleanup**: `try/finally` with `removeTmpDir` (per `.claude/rules/testing.md`)
- **Default timeout**: `setDefaultTimeout(60_000)` for files using subprocess fixtures
- **Startup-check suppression**: `SKILLTAP_NO_STARTUP=1` (handled by `runSkilltap`)

## Existing Coverage

What's already covered (do not duplicate):

| Area | Covered by | Notes |
|---|---|---|
| state.json schema + I/O | `core/state/{schema,io,migrate-v1}.test.ts` | Strong unit coverage |
| Manifest schema + I/O | `core/manifest/{schemas,io,update,publish,range}.test.ts` | Strong |
| Sync drift detection + plan + apply | `core/sync/{drift,plan,apply}.test.ts` | All `DriftKind` variants |
| Migrate run + config-v1 + detect | `core/migrate/{run,config-v1,detect}.test.ts` | All scopes, HTTP-tap abort |
| Try preview (core) | `core/try.test.ts` | Skill/plugin/skip-scan/warnings |
| `composePolicy` agent-mode precedence | `core/policy.test.ts` | All sources + combined ordering |
| `resolveScope` smart-default | `cli/ui/resolve.test.ts` | All 5 branches (added by test-quality work) |
| HTTP-tap stderr warning | `core/taps.http-removal.test.ts` | Subprocess-level |
| Doctor v2.0 checks (unit) | `core/doctor/checks/*.test.ts` | All 6 v2.0 checks |
| Doctor `--json` v2.0 check names | `cli/commands/doctor.test.ts` | Regression catcher |
| install + manifest + lockfile + state.json | `cli/e2e-v2.test.ts:1-2` | One source format only |
| Status dashboard (basic) | `cli/e2e-v2.test.ts:3` | Skill listing |
| Doctor clean run | `cli/e2e-v2.test.ts:4` | Happy path |
| Sync `--apply` on fresh clone | `cli/e2e-v2.test.ts:5` | Cargo-style determinism |
| Migrate v1 installed.json → state.json | `cli/e2e-v2.test.ts:6` | Single-skill v1 fixture |
| `--agent` flag forces non-interactive | `cli/e2e-v2.test.ts:7` | Output format |
| Sync read-only / --json / --apply on in-sync / no project root | `cli/commands/sync.test.ts` | Added by test-quality work |
| Try CLI happy path / --json / never-writes invariant | `cli/commands/try.test.ts` | Added by test-quality work |
| Migrate already-on-v2 / HTTP abort / --json / idempotency | `cli/commands/migrate.test.ts` | Added by test-quality work |
| install never writes installed.json | `cli/e2e-v2.test.ts:2` | Added by test-quality work |

Identified gaps that this design closes:

1. **Plugin install + component toggle journey** — only core-level tests; no end-to-end CLI flow that installs a plugin, toggles a component, and verifies MCP injection updates.
2. **MCP-only install (`mcp:` prefix)** — core unit tests exist; no CLI subprocess test exercises `skilltap install mcp:<source>` and `skilltap remove mcp:<source>`.
3. **Tap journey** — add → list → find → install via shorthand → remove tap. Pieces are tested individually but not as a sequence.
4. **Source-format breadth** — only local-path source is covered in e2e. `github:`, `npm:`, `git:`, `local:` source formats lack end-to-end CLI tests.
5. **Drift workflow at CLI level** — install out-of-band, then sync detects + applies. Currently only fresh-clone reinstall is covered.
6. **Doctor `--fix` repair workflow** — unit tests verify each fixer; no journey test exercises a corrupt/drifted project being repaired by `doctor --fix`.
7. **Update with security scan** — update fetches new content, scan runs, `on_warn` policy is enforced. The scan path is not exercised end-to-end via `update`.
8. **Skill remove drops from manifest + lockfile** — unit-tested via `removeSkillFromManifest`; no CLI subprocess test verifying `skilltap skills remove` updates `skilltap.toml` and `skilltap.lock`.
9. **link / unlink workflow** — `skills link` + `skills unlink` against a local dev directory; status reports linked.
10. **Status `--json` + drift indicators** — only human-readable status tested.
11. **Multi-plugin repo selector** — `repo:plugin-name` and `repo:*` syntaxes; `--agent` mode error when ambiguous.
12. **Failure modes** — invalid TOML, lock-stale recovery, corrupt state.json + doctor --fix, security warnings under each `on_warn` value.

---

## Golden-Path Tests

### Journey: First-time global install (no project, no git)

#### Test 1: Outside any git repo, install defaults to global with no prompt
- **Priority:** High
- **Setup:** Fresh `homeDir` + `configDir` + a `cwd` that's a tmp dir with no `.git` ancestor. Standalone skill fixture repo.
- **Steps:** Run `skilltap install <fixture> --yes --skip-scan` with `cwd` set to the non-git dir.
- **Assertions:** Exit 0; `<homeDir>/.agents/skills/standalone-skill/` exists; `<configDir>/skilltap/state.json` records the skill with `scope: "global"`; `skilltap.toml` is NOT created (no manifest outside a project).
- **Teardown:** Cleanup all temp dirs.

#### Test 2: Outside any project, status reports "global scope" and lists the skill
- **Priority:** Medium
- **Setup:** Continuation of Test 1 environment.
- **Steps:** Run `skilltap status --json`.
- **Assertions:** Exit 0; JSON has `scope: "global"`; `skills` array contains the installed entry; no manifest/lockfile drift indicators.
- **Teardown:** Continuation cleanup.

---

### Journey: Drift detection + reconciliation (out-of-band install)

> Spec: SPEC.md §v2.0 Sync Command (L3144-3168). Tests the read-only inspection
> mode and `--apply` reconciliation against drift introduced manually.

#### Test 3: install + delete state.json out-of-band → sync reports lock-orphan/missing
- **Priority:** High
- **Setup:** Bootstrap a project, install one skill (writes manifest + lockfile + state). Delete `<projectRoot>/.agents/state.json` manually.
- **Steps:** Run `skilltap sync` (read-only).
- **Assertions:** Exit 0; stdout contains "drift report"; the locked-skill entry surfaces as drift; ends with the `skilltap sync --apply` hint.
- **Teardown:** Cleanup project dir.

#### Test 4: sync --apply restores state from lockfile after state-json deletion
- **Priority:** High
- **Setup:** Same as Test 3.
- **Steps:** Run `skilltap sync --apply`.
- **Assertions:** Exit 0; `state.json` re-written with `version: 2`; the skill record is back; install dir on disk exists.
- **Teardown:** Cleanup project dir.

#### Test 5: Manifest declares skill not in state → sync 'add' reinstalls on apply
- **Priority:** High
- **Setup:** Project with manifest declaring a skill, lockfile present, but `state.json` empty (mimics first-clone-after-someone-else-installed).
- **Steps:** Run `skilltap sync --json`, then `skilltap sync --apply`.
- **Assertions:** First call: `inSync: false`, items contains `{ kind: "add" }`. Second call: exit 0, applied count > 0, on-disk state matches manifest.
- **Teardown:** Cleanup.

#### Test 6: State has skill not in manifest → sync 'remove' uninstalls on apply
- **Priority:** Medium
- **Setup:** Project where `state.json` lists a skill that's not in `skilltap.toml` (manifest dropped manually).
- **Steps:** Run `skilltap sync --apply`.
- **Assertions:** Exit 0; the skill is uninstalled (directory gone); state.json no longer references it; manifest unchanged.
- **Teardown:** Cleanup.

#### Test 7: --strict stops on first failure
- **Priority:** Medium
- **Setup:** Project with two `add`-drift items; one fixture path is intentionally invalid (e.g. nonexistent path) so install will fail. Other is valid.
- **Steps:** Run `skilltap sync --apply --strict`.
- **Assertions:** Exit non-zero; only the first item attempted; the second item is not attempted (verify via state.json — only zero or one skill was installed).
- **Teardown:** Cleanup.

---

### Journey: Plugin install + component toggle

> Spec: SPEC.md §v2.0 Project Manifest (`[plugins]` table) + Plugin Detection
> + MCP injection. Plugins bundle skills + MCP servers + agent definitions.

#### Test 8: Install a plugin from .skilltap/<name>.toml repo, components recorded in state
- **Priority:** High
- **Setup:** Fixture repo with `.skilltap/dev-toolkit.toml` declaring `publish = true`, one skill, one stdio MCP server, one agent.
- **Steps:** Run `skilltap install <fixture> --project --yes --skip-scan` in a project dir.
- **Assertions:** Exit 0; `state.json` has a `plugins[]` entry for `dev-toolkit` with all three components active; the skill is symlinked into the `also` agent dirs; the MCP server is injected into the agent's MCP config (e.g. `.claude/settings.json` if `claude-code` in `also`).
- **Teardown:** Cleanup.

#### Test 9: Toggle a component off, then status shows it as inactive and MCP entry pruned
- **Priority:** High
- **Setup:** Continuation of Test 8.
- **Steps:** Run `skilltap toggle dev-toolkit:<server-name>`.
- **Assertions:** Exit 0; `state.json` shows that component with `active: false`; the MCP entry for that server is removed from the agent's MCP config.
- **Teardown:** Cleanup.

#### Test 10: Disable whole plugin, all components inactive, all MCP entries pruned
- **Priority:** High
- **Setup:** Continuation of Test 9.
- **Steps:** Run `skilltap disable dev-toolkit`.
- **Assertions:** Exit 0; all components show `active: false`; all MCP entries from this plugin are pruned; the skill symlink is moved to `.disabled/`.
- **Teardown:** Cleanup.

#### Test 11: Re-enable plugin restores symlinks and MCP entries
- **Priority:** Medium
- **Setup:** Continuation of Test 10.
- **Steps:** Run `skilltap enable dev-toolkit`.
- **Assertions:** Exit 0; components active again; symlink restored from `.disabled/`; MCP entries re-injected.
- **Teardown:** Cleanup.

---

### Journey: MCP-only install (`mcp:<source>` prefix)

> Spec: SPEC.md §v2.0 MCP-Only Install (L3246-3252). The `mcp:` prefix bypasses
> skill/plugin machinery and tracks servers in a separate state.json bucket.

#### Test 12: Install with `mcp:` prefix injects servers without skills/plugins
- **Priority:** High
- **Setup:** Fixture repo with `.skilltap/<name>.toml` containing `[[servers]]` blocks but no `[[skills]]`. Configure `also = ["claude-code"]` in defaults.
- **Steps:** Run `skilltap install mcp:<fixture> --yes --skip-scan`.
- **Assertions:** Exit 0; `state.json.skills` empty; `state.json.plugins` empty; `state.json.mcpServers` (or whatever the canonical bucket is — see `core/src/state/schema.ts`) has the server entries; agent's MCP config has the entries with the `skilltap:` namespace prefix; no skill symlinks created.
- **Teardown:** Cleanup.

#### Test 13: Remove `mcp:<name>` prunes MCP entries from agent config
- **Priority:** High
- **Setup:** Continuation of Test 12.
- **Steps:** Run `skilltap remove mcp:<name> --yes`.
- **Assertions:** Exit 0; the MCP entries are gone from the agent's MCP config; `state.json.mcpServers` no longer references them.
- **Teardown:** Cleanup.

---

### Journey: Tap workflow (add → search → install via shorthand → remove)

> Spec: SPEC.md §`skilltap tap …` commands. Taps are git-only after Phase 31b.

#### Test 14: Add a git tap, list shows it, find returns its skills
- **Priority:** High
- **Setup:** Local git fixture with `tap.json` listing 2 skills.
- **Steps:** Run `skilltap tap add home <fixture-path>` then `skilltap tap list` then `skilltap find <some-keyword> --local`.
- **Assertions:** All three exit 0; tap appears in `tap list`; `find` returns at least one matching skill from the tap.
- **Teardown:** Cleanup.

#### Test 15: Install a skill via tap-shorthand (`tap-name/skill-name`)
- **Priority:** High
- **Setup:** Continuation of Test 14, with a real source repo for the skill.
- **Steps:** Run `skilltap install home/<skill-name> --yes --skip-scan`.
- **Assertions:** Exit 0; the skill is installed; `state.json` records the source as the resolved repo (not the shorthand).
- **Teardown:** Cleanup.

#### Test 16: Remove tap removes config entry and clears tap cache
- **Priority:** Medium
- **Setup:** Continuation of Test 15.
- **Steps:** Run `skilltap tap remove home`.
- **Assertions:** Exit 0; `config.toml`'s `[[taps]]` no longer lists `home`; tap cache directory at `<configDir>/skilltap/taps/home` is removed; previously-installed skill from this tap is unaffected (already in state.json).
- **Teardown:** Cleanup.

---

### Journey: Source-format breadth

> Spec: SPEC.md §`skilltap install` (L7-122) — `<source>` accepts URL, `owner/repo`,
> `npm:`, `local:`, `git:`, or local path. Each adapter must work end-to-end.

#### Test 17: Install via `local:./path` source
- **Priority:** Medium
- **Setup:** Local skill fixture next to a project root.
- **Steps:** Run `skilltap install local:./relative-path --yes --skip-scan` with cwd = project root.
- **Assertions:** Exit 0; manifest's `[skills]` table has key starting with `local:`; install dir exists.
- **Teardown:** Cleanup.

#### Test 18: Install via plain owner/repo shorthand resolves to default git host
- **Priority:** Medium
- **Setup:** Configure `default_git_host` to a local git server fixture (or skip if real github access is required — mark as "requires network" and conditionally skip in CI).
- **Steps:** Run `skilltap install someuser/somerepo --yes --skip-scan`.
- **Assertions:** Exit 0; resolved URL begins with `default_git_host`.
- **Teardown:** Cleanup.

> Note: `npm:` and `git:` source-format tests should be added but require either
> a fixture npm registry or a local git server. Mark those Lower priority pending
> infrastructure; cover at the unit level only for now (already done in
> `core/source-adapters/*.test.ts`).

---

### Journey: link / unlink local development workflow

> Spec: SPEC.md §`skilltap skills link` (L312) — symlink a local skill dir for
> in-place development without commit-and-pull.

#### Test 19: Link a local skill, status reports it as linked
- **Priority:** Medium
- **Setup:** Local skill directory (with `SKILL.md`) outside any git repo.
- **Steps:** Run `skilltap skills link <local-path> --global` then `skilltap status --json`.
- **Assertions:** Exit 0; symlink at `<homeDir>/.agents/skills/<name>` points to the local path; `state.json` entry has `linked: true`; status JSON includes `linked: 1` (or equivalent indicator).
- **Teardown:** Cleanup.

#### Test 20: Unlink a linked skill removes the symlink and state entry
- **Priority:** Medium
- **Setup:** Continuation of Test 19.
- **Steps:** Run `skilltap skills unlink <name>`.
- **Assertions:** Exit 0; symlink is removed; state.json no longer references the skill; the original local source directory is untouched.
- **Teardown:** Cleanup.

---

### Journey: Update fetches and re-runs security scan

> Spec: SPEC.md §`skilltap update` (L265-311) + §Security Scanning (L1728+).
> Update is the surface where stale-content security risks materialize — the
> scan must fire on the new content, and `on_warn` policy must be enforced.

#### Test 21: Update fetches new commit, scans it, applies if clean
- **Priority:** High
- **Setup:** Install a skill from a local git fixture. Commit a benign update to the fixture (modify SKILL.md content but no security signals).
- **Steps:** Run `skilltap update <skill-name> --yes`.
- **Assertions:** Exit 0; `state.json`'s `sha` for the skill changed to the new commit; install dir reflects the new content; stdout mentions the scan running (verify in verbose mode if applicable).
- **Teardown:** Cleanup.

#### Test 22: Update with on_warn=fail (agent mode) blocks update on security warning
- **Priority:** High
- **Setup:** Install a clean skill. Commit an update to the fixture that introduces a static-scan warning (e.g. add `eval(` or `child_process.exec(...)` pattern). Configure `[security.agent].on_warn = "fail"` (default for agent mode).
- **Steps:** Run `skilltap update <skill-name> --agent`.
- **Assertions:** Exit non-zero; clear "blocked" message naming the warning category; install dir unchanged (the skill content reflects the *previous* commit, not the new one); state.json's sha is unchanged.
- **Teardown:** Cleanup.

---

### Journey: Doctor --fix repair workflow

> Spec: SPEC.md §v2.0 Doctor Upgrades (L3254-3263). `doctor --fix` should
> auto-repair: corrupt state.json (regenerate from manifest), missing
> directories, MCP orphans, v0.x file orphans (rename to .v1.bak).

#### Test 23: doctor --fix repairs corrupt state.json
- **Priority:** High
- **Setup:** Project with state.json containing invalid JSON (`{not valid}`).
- **Steps:** Run `skilltap doctor --fix`.
- **Assertions:** Exit 0; state.json is now valid (parses, has `version: 2`); doctor output mentions the fix applied.
- **Teardown:** Cleanup.

#### Test 24: doctor --fix prunes orphan MCP entries from agent config
- **Priority:** Medium
- **Setup:** Plant an `skilltap:nonexistent:server` entry in `<homeDir>/.claude/settings.json`'s `mcpServers` block. No corresponding state.json record.
- **Steps:** Run `skilltap doctor --fix`.
- **Assertions:** Exit 0; the orphan MCP entry is removed from the agent config; doctor output mentions the prune.
- **Teardown:** Cleanup.

#### Test 25: doctor --fix renames v0.x installed.json/plugins.json to .v1.bak when state.json exists
- **Priority:** Medium
- **Setup:** Project with both a valid `state.json` AND a leftover `installed.json` (v0.x orphan, e.g. from a partial migration).
- **Steps:** Run `skilltap doctor --fix`.
- **Assertions:** Exit 0; `installed.json` is gone; `installed.json.v1.bak` exists; state.json untouched.
- **Teardown:** Cleanup.

---

### Journey: Skill remove drops from manifest + lockfile

> Spec: SPEC.md §v2.0 Sync (L2920) — "remove `<pkg>` drops from both [manifest
> and lockfile]." `cli/e2e-v2.test.ts:2` covers install side; remove side has no
> CLI subprocess coverage.

#### Test 26: skills remove drops the entry from manifest, lockfile, and state
- **Priority:** High
- **Setup:** Project with manifest declaring a skill, lockfile entry, state entry, and on-disk install (full Test 2 e2e bootstrap state).
- **Steps:** Run `skilltap skills remove <name> --yes`.
- **Assertions:** Exit 0; `skilltap.toml` no longer lists the skill; `skilltap.lock`'s `[[skill]]` array no longer references it; `state.json` no longer has it; install dir is gone.
- **Teardown:** Cleanup.

---

### Journey: Status dashboard `--json` with drift indicators

> Spec: SPEC.md §v2.0 Status Dashboard (L3178-3210). The `--json` form is "the
> machine-readable equivalent with the same fields" and should include drift
> hints when manifest ↔ state disagrees.

#### Test 27: status --json includes scope, skills, plugins, taps, drift indicators
- **Priority:** Medium
- **Setup:** Project with one installed skill + one tap configured + one declared-but-not-installed manifest entry (synthetic `add` drift).
- **Steps:** Run `skilltap status --json`.
- **Assertions:** Exit 0; output is valid JSON; payload has `skills[]`, `plugins[]`, `taps[]`, and a drift field (whatever the canonical key is — verify in `cli/commands/status.ts`); drift field signals at least one declared-but-uninstalled item.
- **Teardown:** Cleanup.

---

## Adversarial / Failure-Mode Tests

### Category: Invalid manifest / lockfile / state

#### Test A1: Malformed skilltap.toml fails install before any writes
- **Priority:** Critical
- **Scenario:** Project with `skilltap.toml` containing a deliberate TOML syntax error (e.g. unclosed string).
- **Action:** `skilltap install <fixture> --yes --skip-scan`.
- **Expected:** Exit 1; stderr names the parse error and the file path; no state.json written; no install dir created.
- **Verify no side effects:** `state.json` does not exist; `<projectRoot>/.agents/skills/<name>` does not exist.

#### Test A2: skilltap.toml schema mismatch (non-bool component value) errors with prettifyError
- **Priority:** High
- **Scenario:** Manifest with `[plugins]` inline-table where a component value is a string instead of a boolean: `components = { "x" = "true" }` (literal string).
- **Action:** `skilltap sync` (which loads the manifest).
- **Expected:** Exit 1; stderr surfaces a Zod-prettified error citing the offending key; no writes occur.
- **Verify no side effects:** Lockfile + state.json unchanged.

#### Test A3: skilltap.lock with version != 1 is rejected
- **Priority:** High
- **Scenario:** Project with valid manifest but lockfile containing `version = 2`.
- **Action:** `skilltap sync`.
- **Expected:** Exit 1; clear error citing the lockfile version; no auto-upgrade.
- **Verify no side effects:** Lockfile unchanged.

#### Test A4: Corrupt state.json triggers doctor failure (without --fix)
- **Priority:** High
- **Scenario:** Project with `state.json` containing `{not valid}`.
- **Action:** `skilltap doctor` (no --fix).
- **Expected:** Exit non-zero; the `state.json` check reports `fail` with a clear reason; the `--fix` hint is shown.
- **Verify no side effects:** state.json untouched (no auto-repair without --fix).

---

### Category: Bad environment

#### Test A5: Source URL unreachable (network failure)
- **Priority:** High
- **Scenario:** Pass a `git:` URL pointing to a host that doesn't resolve (e.g. `git://this-host-does-not-exist.invalid/x`).
- **Action:** `skilltap install <bad-url> --yes --skip-scan`.
- **Expected:** Exit 1; stderr surfaces the `GitError` with the underlying git stderr; no half-installed state.
- **Verify no side effects:** `state.json` unchanged; no skill dir created; no manifest entry written.

#### Test A6: Read-only home directory fails with clear error
- **Priority:** Medium
- **Scenario:** `chmod 0500 <homeDir>` before running install.
- **Action:** `skilltap install <fixture> --yes --skip-scan`.
- **Expected:** Exit 1; stderr surfaces a permission error; no partial state written.
- **Verify no side effects:** No files exist under the read-only home.
- **Note:** Skip on Windows. May need to be POSIX-only.

#### Test A7: HTTP tap in v0.x config is silently filtered with stderr warning
- **Priority:** Medium *(already covered by `core/taps.http-removal.test.ts` — list here for completeness; do not duplicate the test)*
- **Scenario:** Config with `[[taps]]` entry of `type = "http"` plus a working git tap.
- **Action:** Any command that loads taps (e.g. `skilltap tap list`).
- **Expected:** Exit 0; stderr emits the one-time "HTTP tap '<name>' ignored" warning naming the tap; only the git tap appears in the result.
- **Verify no side effects:** Config file untouched (the filter is read-only).

---

### Category: Boundary conditions

#### Test A8: Empty source argument errors with usage hint
- **Priority:** Medium
- **Scenario:** `skilltap install` with no source arg.
- **Action:** `skilltap install --yes`.
- **Expected:** Exit non-zero; stderr mentions the missing source positional + usage line.
- **Verify no side effects:** No state writes.

#### Test A9: Source repo with no SKILL.md errors with "no skills found"
- **Priority:** High
- **Scenario:** Local git fixture that is a valid repo but has no `SKILL.md` anywhere.
- **Action:** `skilltap install <fixture> --yes --skip-scan`.
- **Expected:** Exit 1; stderr mentions "No skills found" or equivalent; the source is reported.
- **Verify no side effects:** No state writes; tmp clone cleaned up.

#### Test A10: Multi-plugin repo without selector errors in --agent mode
- **Priority:** High
- **Scenario:** Repo with two `.skilltap/<name>.toml` files both `publish = true`.
- **Action:** `skilltap install <fixture> --agent --yes --skip-scan`.
- **Expected:** Exit 1; stderr says "multiple plugins available: <name1>, <name2>; specify with `<source>:<name>`"; no install occurs.
- **Verify no side effects:** No state.json change.

#### Test A11: Empty skilltap.toml is valid (all defaults applied)
- **Priority:** Medium
- **Scenario:** Project with a zero-byte `skilltap.toml`.
- **Action:** `skilltap sync` then `skilltap install <fixture> --yes --skip-scan`.
- **Expected:** Sync reports in-sync (empty manifest, empty lockfile, empty state). Install succeeds and writes the skill into the manifest.
- **Verify:** Manifest now has one skill entry; lockfile has one entry.

#### Test A12: Skill name with regex-rejected characters fails plugin manifest parse
- **Priority:** Low
- **Scenario:** Plugin manifest with `[[skills]]` entry whose `name = "Bad_Name"` (caps + underscore violate the `^[a-z0-9]+(-[a-z0-9]+)*$` regex).
- **Action:** `skilltap install <fixture> --yes --skip-scan`.
- **Expected:** Exit 1; schema error citing the regex violation.
- **Verify no side effects:** No partial install.

---

### Category: Migration safety

#### Test A13: Migrate aborts on HTTP taps without writing state.json
- **Priority:** Critical *(already covered by `cli/commands/migrate.test.ts` — list here for completeness; do not duplicate)*
- **Scenario:** v1 environment with `installed.json` + a `[[taps]] type="http"` entry in config.toml.
- **Action:** `skilltap migrate`.
- **Expected:** Exit 1; stderr lists the offending HTTP tap names; state.json is NOT written; .v1.bak renames are NOT done.
- **Verify no side effects:** All v1 files exactly as they were.

#### Test A14: Migrate is idempotent — second run is a no-op
- **Priority:** High *(already covered by `cli/commands/migrate.test.ts` — list here for completeness)*
- **Scenario:** v1 environment, run migrate once successfully, run migrate again.
- **Action:** Two consecutive `skilltap migrate` invocations.
- **Expected:** First run migrates and writes .v1.bak. Second run reports "Already on v2.0" and exits 0.
- **Verify:** .v1.bak files are not double-renamed; state.json is not corrupted.

---

### Category: Security policy enforcement

#### Test A15: install fires static scan; on_warn=fail blocks install with clear error
- **Priority:** High
- **Scenario:** Skill fixture with a static-scan trigger (e.g. `eval(...)` in SKILL.md content) — see `core/security/detectors/*.ts` for triggers. Configure `[security.human].on_warn = "fail"`.
- **Action:** `skilltap install <fixture> --yes` (no --skip-scan).
- **Expected:** Exit 1; stderr surfaces the warning category + file:line; "blocked by security policy" or equivalent message.
- **Verify no side effects:** No state.json write; no install dir created.

#### Test A16: install with --skip-scan is rejected when require_scan=true
- **Priority:** High
- **Scenario:** Configure `[security.human].require_scan = true`. Use a clean fixture.
- **Action:** `skilltap install <fixture> --skip-scan --yes`.
- **Expected:** Exit 1; stderr says scan is required by config; no install occurs.
- **Verify no side effects:** No state.json write.

#### Test A17: --agent mode uses [security.agent] policy (default on_warn=fail)
- **Priority:** High
- **Scenario:** Skill fixture with a static-scan trigger. Default agent-mode policy (`on_warn=fail`, `require_scan=true`).
- **Action:** `skilltap install <fixture> --agent` (no --yes, no --skip-scan).
- **Expected:** Exit 1; agent-mode plain-text "blocked" message; no install.
- **Verify no side effects:** No state.json write.

---

### Category: State and lockfile consistency

#### Test A18: Lock-stale (sha mismatch) drift surfaces and resolves on apply
- **Priority:** High
- **Scenario:** Install a skill, then manually edit the lockfile so the `sha` for that entry differs from the on-disk sha.
- **Action:** `skilltap sync` (read-only) then `skilltap sync --apply`.
- **Expected:** First call: drift report includes a `lock-stale` item. Second call: exit 0, the on-disk skill is reinstalled to match the lockfile sha.
- **Verify:** Post-apply, on-disk sha matches lockfile sha.

#### Test A19: Lock-orphan (lockfile entry, no manifest, no state) is dropped on apply
- **Priority:** Medium
- **Scenario:** Lockfile contains an entry that's neither in manifest nor in state.json.
- **Action:** `skilltap sync --apply`.
- **Expected:** Exit 0; the orphan lockfile entry is removed; nothing is installed.
- **Verify:** Lockfile no longer references the orphan.

#### Test A20: Concurrent install — second invocation while first is running
- **Priority:** Low *(stretch goal — file locking is not currently designed)*
- **Scenario:** Start `skilltap install <large-fixture>` in the background; immediately run `skilltap install <other-fixture>` against the same scope.
- **Action:** Two concurrent invocations.
- **Expected:** Both succeed (state.json is written atomically), OR the second fails clearly with a lock error. Either is acceptable; the failure mode that's NOT acceptable is a corrupted/partially-written state.json.
- **Verify:** Final `state.json` parses correctly; both installs are reflected (or neither is corrupted if one was rejected).
- **Note:** This test may be flaky on slow CI; mark as known-flaky if so.

---

## Implementation Notes

- **All tests should use `runSkilltap`** (subprocess pipe mode) unless the test explicitly exercises clack-rendered prompts; in that case use `runInteractive`. See `.claude/rules/testing.md`.
- **Per-test env isolation** via `createTestEnv()` from `@skilltap/test-utils` — prevents state leak between tests.
- **Existing tests are the substrate.** Several adversarial tests above (A7, A13, A14) are already covered. The design lists them for completeness but the implementation effort focuses on the un-covered tests.
- **Fixture additions needed:**
  - `createPluginFixtureRepo(opts)` — creates a `.skilltap/<name>.toml` repo with declarable `[[skills]]`, `[[servers]]`, `[[agents]]`. Useful for J5, J7, A10, A12.
  - `createMcpOnlyFixtureRepo()` — like above but only `[[servers]]`. For J7.
  - `createSkillRepoWithSecurityWarning()` — clean skill body but contains a known-trigger pattern (e.g. `eval(`). For A15, A17.
  - `corruptStateJson(projectRoot)` — helper to plant invalid JSON. For A4, T23.
  - `mutateLockfileSha(projectRoot, name, newSha)` — helper for A18.
- **New helper for source-format tests:** A small local git server (or `file://` URLs) to exercise `git:` adapter without real network. Defer until needed; for now the unit-level adapter coverage is sufficient.
- **Test file layout:**
  - Put plugin lifecycle journey (J5 = T8-11) in `cli/src/commands/plugin/lifecycle.test.ts` (new file).
  - Put MCP-only journey (J6 = T12-13) in `cli/src/commands/install.mcp.test.ts` (new file).
  - Put tap journey (J9 = T14-16) in `cli/src/commands/tap.journey.test.ts` (new file).
  - Put doctor --fix journey (J11 = T23-25) in `cli/src/commands/doctor.fix.test.ts` (new file, separate from existing `doctor.test.ts`).
  - Drift workflow (J4 = T3-7) extends `cli/src/commands/sync.test.ts` (added by test-quality work).
  - Update + security (J10 = T21-22) extends `cli/src/commands/update.test.ts`.
  - link/unlink (J12 = T19-20) — new file `cli/src/commands/skills/link.journey.test.ts`.
  - Adversarial tests live next to the related happy-path file.
- **Don't use `--skip-scan` as a crutch** for tests that are about the scan path (per `.claude/rules/testing.md`). A15-A17 must run the real scanner; do not pass --skip-scan.
- **`SKILLTAP_NO_STARTUP=1`** is set automatically by `runSkilltap` — suppresses the v1-detection startup notice that would otherwise pollute stderr.
- **Avoid `--no-*` flag names** (`mri` intercepts them). Tests should use the explicit positive form: `--quiet`, `--force`, etc.

## Priority Order

Highest-value tests first. Implement top-to-bottom; ship after Critical + High tier.

1. **Critical** — A1 (malformed manifest), A13 (migrate HTTP abort, already done), T23 (doctor --fix corrupt state)
2. **High — manifest-driven journeys (the v2.0 story)**
   - T3-T6 (drift detection + apply)
   - T26 (skill remove drops from manifest+lockfile)
   - T8-T10 (plugin install + toggle + disable)
3. **High — never-writes / safety invariants** (mostly covered already by test-quality work; verify in CI)
   - T1 (first-time global install)
   - A14 (migrate idempotency, already done)
4. **High — security policy**
   - T22 (update with on_warn=fail blocks)
   - A15-A17 (install scan policy enforcement)
5. **High — adversarial state**
   - A2 (schema mismatch with prettifyError)
   - A3 (lockfile version rejection)
   - A4 (corrupt state.json without --fix)
   - A18 (lock-stale recovery)
6. **Medium — workflow journeys**
   - T12-T13 (mcp:source install/remove)
   - T14-T16 (tap journey)
   - T17 (local: source format)
   - T19-T20 (link/unlink)
   - T21 (update happy path)
   - T24 (doctor --fix MCP orphan prune)
   - T25 (doctor --fix v0.x rename)
   - T27 (status --json)
7. **Medium — adversarial**
   - A5 (network failure)
   - A6 (read-only home, POSIX only)
   - A8-A11 (boundary conditions)
   - A19 (lock-orphan drop)
8. **Low — stretch / infrastructure-dependent**
   - T11 (re-enable plugin)
   - T18 (owner/repo shorthand against real network)
   - A12 (regex rejection)
   - A20 (concurrent install — known-flaky candidate)
   - `npm:` and `git:` source-format e2e (need test infrastructure)

## Out of Scope

These belong in other test layers and are intentionally not in this design:

- **Schema-level validation** (manifest schemas, lockfile schema, state schema) — already strong unit coverage in `core/manifest/schemas.test.ts`, `core/state/schema.test.ts`. Don't duplicate at e2e.
- **Pure drift-detection logic** — `core/sync/drift.test.ts` covers all `DriftKind` variants exhaustively. E2E tests use drift output as a black-box signal, not a unit assertion.
- **`composePolicy` precedence** — exhaustively tested in `core/policy.test.ts` after the test-quality work. E2E tests assume the function is correct.
- **PTY-rendered interactive flows** — covered in `cli/src/{interactive,commands/find.interactive,commands/tap.install.interactive}.test.ts`. The v2.0 e2e suite is pipe-mode (`runSkilltap`); PTY tests live in their own files because they have different infrastructure (node-pty bridge).
