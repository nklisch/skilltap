# E2E Test Suite Design: Orphan Handling & Marketplace.json

## Project Summary

skilltap CLI tool. Two recent features need E2E coverage:
1. **Orphan/state coherence handling** — detect and clean up stale installed.json records when filesystem state diverges
2. **marketplace.json as tap format** — Claude Code marketplace repos recognized as taps

## Test Environment

- **Framework**: `bun:test` (subprocess via `runSkilltap`, PTY via `runInteractive`)
- **Fixtures**: `createStandaloneSkillRepo`, `createMultiSkillRepo`, `makeTmpDir`, `initRepo`, `commitAll`
- **Env isolation**: `SKILLTAP_HOME` + `XDG_CONFIG_HOME` per test
- **Cleanup**: `try/finally` with `removeTmpDir`
- **Timeout**: `setDefaultTimeout(60_000)`

## Existing Coverage

- 10 update subprocess tests + 5 agent-mode update tests (none cover orphan scenarios)
- 16 install subprocess tests (none cover phantom conflicts)
- 17 tap tests (none test marketplace.json format)
- 0 tests for scanner plugin layout discovery via CLI

---

## Golden-Path Tests

### Journey: Orphan Auto-Cleanup During Update

#### Test 1: Agent mode auto-cleans orphan record during update
- **Priority:** High
- **Setup:** Install standalone skill, delete its install directory
- **Steps:** Run `skilltap update` in agent mode (pipe stdin)
- **Assertions:** Exit 0, stdout contains "Stale record" warning and "Auto-removing", no crash
- **Teardown:** Cleanup temp dirs

#### Test 2: --yes flag auto-cleans orphan record during update
- **Priority:** High
- **Setup:** Install standalone skill, delete its install directory
- **Steps:** Run `skilltap update --yes`
- **Assertions:** Exit 0, stale record cleaned, healthy skills still update
- **Teardown:** Cleanup temp dirs

### Journey: Phantom Conflict Resolution During Install

#### Test 3: Agent mode installs through phantom conflict
- **Priority:** High
- **Setup:** Write stale record in installed.json for "my-skill" (no directory on disk), create git repo with "my-skill"
- **Steps:** Run `skilltap install <repo-path> --global` in agent mode
- **Assertions:** Exit 0, skill installed successfully, stale record replaced with new one
- **Teardown:** Cleanup repos + temp dirs

### Journey: Remove Orphaned Skill

#### Test 4: Remove succeeds when directory already missing
- **Priority:** Medium
- **Setup:** Install skill, delete its directory
- **Steps:** Run `skilltap skills remove my-skill --yes --global`
- **Assertions:** Exit 0, record removed from installed.json, output mentions cleanup
- **Teardown:** Cleanup temp dirs

### Journey: Marketplace Tap Add

#### Test 5: Tap add with marketplace.json repo
- **Priority:** High
- **Setup:** Create git repo with `.claude-plugin/marketplace.json` listing 2 plugins
- **Steps:** Run `skilltap tap add test-marketplace <repo-path>`
- **Assertions:** Exit 0, stdout contains skill count (2), tap appears in `tap list`
- **Teardown:** Cleanup repos

#### Test 6: Install from marketplace-sourced tap
- **Priority:** High
- **Setup:** Create marketplace tap + standalone skill repos it references, add tap
- **Steps:** Run `skilltap install <skill-name> --global --yes`
- **Assertions:** Exit 0, skill installed, recorded in installed.json with tap reference
- **Teardown:** Cleanup repos + temp dirs

### Journey: Multi-Skill Update with Mixed Health

#### Test 7: Update with one orphan and one healthy skill
- **Priority:** High
- **Setup:** Install 2 standalone skills, delete one's directory, add new commit to the other
- **Steps:** Run `skilltap update --yes`
- **Assertions:** Exit 0, orphan cleaned (warning printed), healthy skill updated
- **Teardown:** Cleanup repos

### Journey: Plugin Layout Scanner

#### Test 8: Install from repo with plugins/*/skills/*/SKILL.md layout
- **Priority:** Medium
- **Setup:** Create git repo with `plugins/my-plugin/skills/my-skill/SKILL.md`
- **Steps:** Run `skilltap install <repo-path> --global --yes`
- **Assertions:** Exit 0, skill found and installed, appears in installed.json
- **Teardown:** Cleanup repos

---

## Adversarial / Failure-Mode Tests

### Category: Upstream Restructuring

#### Test 9: Multi-skill cache subdirectory removed upstream (THE crash fix)
- **Priority:** Critical
- **Scenario:** Install from multi-skill repo (2 skills). Upstream removes one skill's subdirectory and commits. Run update.
- **Action:** `skilltap update --yes`
- **Expected:** Exit 0 (NOT crash), removed skill reported as "removed from upstream", surviving skill updated normally
- **Verify:** No cp error in stderr, installed.json has only the surviving skill

### Category: Complete State Corruption

#### Test 10: All installed skills are orphaned
- **Scenario:** Install 2 skills, delete BOTH directories
- **Action:** `skilltap update --yes`
- **Expected:** Exit 0, both stale records cleaned, output shows warnings for both
- **Verify:** installed.json is empty after cleanup

### Category: Manual Deletion + Re-install

#### Test 11: Re-install skill after manually deleting its directory
- **Scenario:** Install skill, rm -rf the install dir (but leave installed.json record)
- **Action:** `skilltap install <same-repo> --global --yes`
- **Expected:** Exit 0, fresh install succeeds (no "already installed" conflict)
- **Verify:** installed.json has exactly one record, directory exists

### Category: Idempotency

#### Test 12: Orphan cleanup is idempotent — running update twice
- **Scenario:** Install skill, delete directory
- **Action:** Run `skilltap update --yes` twice in sequence
- **Expected:** First run: warns and cleans. Second run: clean pass, no warnings, exit 0
- **Verify:** Both runs exit 0

### Category: Multiple Orphan Types

#### Test 13: Mixed orphan types cleaned in one update
- **Scenario:** Install a standalone skill (delete dir → directory-missing) + a linked skill (delete target → link-target-missing)
- **Action:** `skilltap update --yes`
- **Expected:** Both orphans detected and cleaned, exit 0
- **Verify:** installed.json has no stale records

### Category: Agent Mode Multi-Skill Cache

#### Test 14: Agent mode with stale multi-skill record (cache completely gone)
- **Scenario:** Install from multi-skill repo, delete the entire git cache directory
- **Action:** `skilltap update` in agent mode
- **Expected:** Warns about stale records, auto-removes, exits 0
- **Verify:** No crash, installed.json cleaned

### Category: Empty Marketplace

#### Test 15: Tap add with marketplace.json — empty plugins array
- **Scenario:** Create marketplace repo with `plugins: []`
- **Action:** `skilltap tap add empty-market <repo-path>`
- **Expected:** Exit 0, reports 0 skills
- **Verify:** Tap appears in `tap list` with 0 skills

---

## Implementation Notes

- All tests use `runSkilltap` (subprocess) since orphan handling is non-interactive in agent mode / `--yes` mode
- Agent mode is triggered by piping stdin (making `isTTY = false`)
- Multi-skill fixtures need `createMultiSkillRepo` or manual creation with `.agents/skills/` layout
- For test 9 (THE crash fix), need to create a multi-skill repo, install, then modify the repo to remove a skill's subdirectory and commit
- `disableBuiltinTap` helper needed to prevent network calls to builtin tap during tests
- Use `SKILLTAP_NO_STARTUP=1` env var (handled by `runSkilltap`) to suppress startup checks

## Priority Order

1. **Test 9** — THE crash fix (Critical)
2. **Tests 1-3** — Core orphan auto-cleanup (High)
3. **Test 7** — Mixed health update (High)
4. **Tests 5-6** — Marketplace tap (High)
5. **Tests 11-12** — Re-install + idempotency (High)
6. **Tests 10, 13-14** — Edge cases (Medium)
7. **Tests 4, 8, 15** — Minor journeys (Medium)
