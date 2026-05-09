# Design: Comprehensive Orphan and State Coherence Handling

## Overview

Every data relationship in skilltap — installed.json ↔ disk, installed.json ↔ git cache, tap.json ↔ repo, symlinks ↔ targets — can go out of sync. The current code assumes coherence and crashes when it's wrong. This design adds **verify-before-act** checks at every state-changing touchpoint so misalignments are detected, reported, and cleaned up instead of causing hard failures.

### Failure Modes Addressed

| # | Mismatch | Current behavior | Example error |
|---|----------|-----------------|---------------|
| 1 | installed.json record → skill directory missing on disk | `update` crashes on git fetch in nonexistent dir; `install` sees false conflict | `cp: cannot stat '/path/to/skill': No such file or directory` |
| 2 | installed.json record → git cache missing (multi-skill) | `update` silently skips all skills in group | No error, but no feedback either |
| 3 | installed.json record → skill subdirectory missing from cache (multi-skill, repo restructured) | `update` crashes on cp | `cp: cannot stat '.../cache/hash/.agents/skills/design-principles': No such file or directory` |
| 4 | Broken agent symlinks → target deleted | Silent broken links, agent can't find skill | Agent silently ignores skill |
| 5 | Linked skill → target path no longer exists | `update` skips; `list` shows stale entry | No error, stale data shown |

### Design Principles

- **No assumptions of coherence.** Every operation that reads from installed.json and then touches the filesystem verifies the path exists before acting.
- **Detect at state-changing touchpoints:** `install`, `update`, `remove`. Not at read-only commands (list/search).
- **Interactive mode:** warn the user, prompt to clean up.
- **Agent mode:** auto-clean orphan records (the directory is already gone — the record is the lie).
- **Callback-driven:** core functions expose `onOrphan` callbacks; CLI layer decides what to do.

## Implementation Units

### Unit 1: Core Orphan Detection Utility

**File**: `packages/core/src/orphan.ts`

```typescript
import type { InstalledJson, InstalledSkill } from "./schemas/installed";
import type { Result } from "./types";

export type OrphanRecord = {
  record: InstalledSkill;
  reason:
    | "directory-missing"      // #1: skill install dir doesn't exist
    | "cache-missing"          // #2: multi-skill git cache dir doesn't exist
    | "cache-subdir-missing"   // #3: skill subdirectory missing from cache
    | "link-target-missing";   // #5: linked skill target gone
};

/**
 * Scan installed.json for records whose corresponding filesystem state is missing.
 * Pure verification — does not modify anything.
 */
export async function findOrphanRecords(
  installed: InstalledJson,
  projectRoot?: string,
): Promise<OrphanRecord[]>;

/**
 * Remove orphan records from installed data and save.
 * Returns the names of removed records.
 */
export async function purgeOrphanRecords(
  orphans: OrphanRecord[],
  installed: InstalledJson,
  fileRoot?: string,
): Promise<Result<string[], import("./types").UserError>>;
```

**Implementation Notes**:

`findOrphanRecords()` iterates `installed.skills` and checks:

1. **Linked skills** (`scope === "linked"`): check `record.path` exists via `resolvedDirExists()`. If missing → `"link-target-missing"`.

2. **Standalone git skills** (`path === null`, repo not npm): check `skillInstallDir(name, scope)` exists (or `skillDisabledDir` if `active === false`). If missing → `"directory-missing"`.

3. **Multi-skill git skills** (`path !== null`, repo not npm):
   - Check git cache at `skillCacheDir(record.repo)/.git` exists. If missing → `"cache-missing"`.
   - If cache exists, check `join(skillCacheDir(record.repo), record.path)` exists. If missing → `"cache-subdir-missing"`.
   - Also check `skillInstallDir(name, scope)` exists. If missing → `"directory-missing"`.

4. **npm skills** (`repo.startsWith("npm:")`): check `skillInstallDir(name, scope)` exists. If missing → `"directory-missing"`.

5. **Local skills** (`repo === null`, not linked): check `skillInstallDir(name, scope)` exists. If missing → `"directory-missing"`.

`purgeOrphanRecords()`:
- Filters `installed.skills` to remove the orphan records
- Calls `saveInstalled()`
- Also removes any agent symlinks for the purged records via `removeAgentSymlinks()`
- Returns the list of purged skill names

**Acceptance Criteria**:
- [ ] Detects missing install directories
- [ ] Detects missing git cache directories (multi-skill)
- [ ] Detects missing subdirectories within git cache (the exact failure the user reported)
- [ ] Detects broken linked skill targets
- [ ] `purgeOrphanRecords` removes records and saves installed.json
- [ ] `purgeOrphanRecords` cleans up agent symlinks for removed records

---

### Unit 2: Orphan Callback Types

**File**: `packages/core/src/types.ts` (additions)

No new file — add to existing types or keep in `orphan.ts`.

The `OrphanRecord` type is already defined in Unit 1. The callback signature used by state-changing functions:

```typescript
/** Called when orphan records are detected before the main operation.
 *  Return the names of records to purge. Return [] to skip cleanup. */
export type OnOrphansFound = (orphans: OrphanRecord[]) => Promise<string[]>;
```

This goes in `orphan.ts` alongside the types.

**Acceptance Criteria**:
- [ ] Type is exported from `@skilltap/core`

---

### Unit 3: Wire Orphan Detection into `updateSkill()`

**File**: `packages/core/src/update.ts`

**Changes:**

1. Add `onOrphansFound?: OnOrphansFound` to `UpdateOptions`.

2. At the top of `updateSkill()`, after loading installed.json (line ~618), call `findOrphanRecords()` on each loaded installed set. If orphans found:
   - Call `options.onOrphansFound(orphans)` to get which to purge
   - Call `purgeOrphanRecords()` for those
   - Filter the purged names out of `globalSkills` / `projectSkills` before proceeding

3. **Fix the crash in `updateGitSkillGroup()`** (line ~480): Before `recopyMultiSkill()`, check that `join(workDir, skill.path!)` exists. If not, report as `"skipped"` with a new progress status `"removed-upstream"` instead of crashing on cp.

```typescript
// In UpdateOptions:
onOrphansFound?: OnOrphansFound;

// New progress status:
onProgress?: (
  skillName: string,
  status: "checking" | "upToDate" | "updated" | "skipped" | "linked" | "local" | "removed-upstream",
) => void;

// Called when a multi-skill's subdirectory is gone from the cache after pull:
onSkillRemovedUpstream?: (skillName: string, repoUrl: string) => Promise<"remove" | "skip">;
```

**Implementation Notes**:

The `onSkillRemovedUpstream` callback handles the exact crash the user reported. In `updateGitSkillGroup()`, after pull and before recopy:

```typescript
// Check if skill still exists in pulled repo
const skillSrcExists = await resolvedDirExists(join(workDir, skill.path!));
if (!skillSrcExists) {
  // Skill was removed from upstream repo
  if (options.onSkillRemovedUpstream) {
    const action = await options.onSkillRemovedUpstream(skill.name, repo);
    if (action === "remove") {
      // Remove from installed.json, remove install dir, remove symlinks
      const installDir = skillInstallDir(skill.name, skill.scope as "global" | "project", options.projectRoot);
      await wrapShell(() => $`rm -rf ${installDir}`.quiet().then(() => undefined), "");
      await removeAgentSymlinks(skill.name, skill.also, skill.scope as "global" | "project", options.projectRoot);
      installed.skills = installed.skills.filter(s => s !== skill);
    }
  }
  result.skipped.push(skill.name);
  options.onProgress?.(skill.name, "removed-upstream");
  continue;
}
```

Also add a similar check in `updateGitSkill()` — after `fetch()` succeeds but the install directory itself doesn't exist (standalone skill whose dir was deleted):

```typescript
// After fetch, before revParse — verify workDir actually exists
if (!(await resolvedDirExists(workDir))) {
  // Orphan: record exists but directory is gone
  result.skipped.push(record.name);
  options.onProgress?.(record.name, "removed-upstream");
  return ok(undefined);
}
```

Wait — for standalone git skills, the workDir IS the install dir, and `fetch()` would already fail if it doesn't exist. So the existing error path catches it. But the error is unhelpful. Instead, check before fetch:

```typescript
// Before fetch: verify the work directory exists
if (!(await resolvedDirExists(workDir))) {
  result.skipped.push(record.name);
  options.onProgress?.(record.name, "removed-upstream");
  return ok(undefined);
}
```

**Acceptance Criteria**:
- [ ] `updateSkill()` detects orphan records before starting and calls `onOrphansFound`
- [ ] Purged orphans are excluded from the update pass
- [ ] Multi-skill update does NOT crash when a skill's subdirectory is missing from cache — calls `onSkillRemovedUpstream` instead
- [ ] Standalone git update does NOT crash when install directory is missing — reports `"removed-upstream"`
- [ ] All existing update tests still pass

---

### Unit 4: Wire Orphan Detection into `installSkill()`

**File**: `packages/core/src/install.ts`

**Changes:**

1. Add `onOrphansFound?: OnOrphansFound` to `InstallOptions`.

2. In the conflict detection loop (line ~516), when a conflict is found in installed.json, verify the directory actually exists before treating it as a real conflict:

```typescript
const conflict = installed.skills.find(
  (s) => s.name === skill.name && s.scope === options.scope,
);
if (conflict) {
  // Verify the "conflict" is real — does the directory actually exist?
  const conflictDir = conflict.active === false
    ? skillDisabledDir(conflict.name, options.scope!, projectRoot)
    : skillInstallDir(conflict.name, options.scope!, projectRoot);

  if (!(await resolvedDirExists(conflictDir))) {
    // Phantom conflict: record exists but directory is gone. Clean up the stale record.
    installed.skills = installed.skills.filter(s => s !== conflict);
    await removeAgentSymlinks(conflict.name, conflict.also, options.scope!, projectRoot);
    toInstall.push(skill);  // proceed with fresh install
    continue;
  }
  // ... existing conflict handling
}
```

3. Also run the full `findOrphanRecords()` check at the top (after loading installed.json) and call `onOrphansFound` if orphans exist — same pattern as update. This handles the case where a user runs `install` and there are stale records for OTHER skills polluting the state.

**Acceptance Criteria**:
- [ ] `install` does not report false conflicts for skills whose directory is missing
- [ ] Stale conflict records are auto-cleaned (record removed, symlinks removed)
- [ ] Full orphan scan runs before install and calls `onOrphansFound`
- [ ] All existing install tests still pass

---

### Unit 5: Wire Orphan Awareness into `removeSkill()`

**File**: `packages/core/src/remove.ts`

**Changes:**

The existing `removeSkill()` already handles missing directories gracefully (rm -rf is a no-op). But the UX is silent — the user doesn't know the directory was already gone. Add logging:

1. Add `onOrphanRemoved?: (name: string) => void` to `RemoveOptions`.

2. Before removing the directory, check if it exists. If not, call `onOrphanRemoved` so the CLI can inform the user:

```typescript
const dirExists = await resolvedDirExists(installPath);
if (!dirExists) {
  options.onOrphanRemoved?.(name);
}
// ... proceed with removal (rm -rf is safe on nonexistent paths)
```

This is a minor improvement — remove already works, it just doesn't tell the user.

**Acceptance Criteria**:
- [ ] `remove` succeeds even when directory is missing (no change from current)
- [ ] `onOrphanRemoved` callback is called when removing a skill whose directory was already gone
- [ ] All existing remove tests still pass

---

### Unit 6: CLI Layer — Interactive Mode Handlers

**File**: `packages/cli/src/commands/update.ts`

**Changes for interactive mode `onOrphansFound` handler:**

```typescript
async onOrphansFound(orphans) {
  if (orphans.length === 0) return [];

  log.warn(`Found ${orphans.length} stale record(s) in installed.json:`);
  for (const o of orphans) {
    log.warn(`  ${o.record.name}: ${formatOrphanReason(o.reason)}`);
  }

  const shouldClean = await confirm({
    message: `Remove these stale records? (directories are already gone)`,
    initialValue: true,
  });

  if (isCancel(shouldClean) || !shouldClean) return [];
  return orphans.map(o => o.record.name);
}
```

**Changes for interactive mode `onSkillRemovedUpstream` handler:**

```typescript
async onSkillRemovedUpstream(skillName, repoUrl) {
  log.warn(`Skill "${skillName}" was removed from the upstream repo.`);
  const action = await confirm({
    message: `Remove "${skillName}" from installed.json?`,
    initialValue: true,
  });
  return (isCancel(action) || !action) ? "skip" : "remove";
}
```

**Changes for `"removed-upstream"` progress status:**

```typescript
// In the onProgress handler:
case "removed-upstream":
  s.message(`${skillName}: removed from upstream repo`);
  break;
```

**File**: `packages/cli/src/commands/install.ts`

Same `onOrphansFound` handler pattern. Use the interactive version if not `--yes`, auto-clean if `--yes`.

**File**: `packages/cli/src/commands/skills/remove.ts`

Add `onOrphanRemoved` handler:
```typescript
onOrphanRemoved(name) {
  log.info(`Note: "${name}" directory was already missing — cleaning up record only.`);
}
```

**Acceptance Criteria**:
- [ ] Interactive update shows orphan list and prompts to clean
- [ ] Interactive update shows "removed from upstream" for restructured repos
- [ ] Interactive install shows orphan list and prompts to clean
- [ ] Remove tells user when directory was already gone
- [ ] `--yes` flag auto-cleans orphans without prompting

---

### Unit 7: CLI Layer — Agent Mode Handlers

**File**: `packages/cli/src/commands/update.ts` (agent mode section)

```typescript
// Agent mode: auto-clean orphan records
async onOrphansFound(orphans) {
  if (orphans.length === 0) return [];
  for (const o of orphans) {
    process.stdout.write(
      `warning: Stale record "${o.record.name}" — ${formatOrphanReason(o.reason)}. Auto-removing.\n`
    );
  }
  return orphans.map(o => o.record.name);  // auto-clean all
},

async onSkillRemovedUpstream(skillName) {
  process.stdout.write(
    `warning: "${skillName}" removed from upstream repo. Auto-removing record.\n`
  );
  return "remove";
},
```

**File**: `packages/cli/src/commands/install.ts` (agent mode section)

Same pattern — auto-clean in agent mode.

**Acceptance Criteria**:
- [ ] Agent mode auto-cleans orphan records with warning messages
- [ ] Agent mode auto-removes skills deleted from upstream repos
- [ ] No prompts in agent mode

---

### Unit 8: Shared Reason Formatter

**File**: `packages/core/src/orphan.ts` (addition)

```typescript
export function formatOrphanReason(reason: OrphanRecord["reason"]): string {
  switch (reason) {
    case "directory-missing":
      return "install directory missing from disk";
    case "cache-missing":
      return "git cache directory missing";
    case "cache-subdir-missing":
      return "skill subdirectory removed from upstream repo";
    case "link-target-missing":
      return "symlink target no longer exists";
  }
}
```

**Acceptance Criteria**:
- [ ] All 4 reason types produce human-readable strings

---

## Implementation Order

1. **Unit 1**: `orphan.ts` — core detection + purge utilities
2. **Unit 2**: Types (in `orphan.ts`)
3. **Unit 8**: `formatOrphanReason` (in `orphan.ts`)
4. **Unit 3**: Wire into `update.ts` — fix the crash, add callbacks
5. **Unit 4**: Wire into `install.ts` — fix phantom conflicts
6. **Unit 5**: Wire into `remove.ts` — add orphan feedback
7. **Unit 6**: CLI interactive handlers
8. **Unit 7**: CLI agent mode handlers

Units 1-3 are the critical path — they fix the crash and detect problems.

## Testing

### Unit Tests: `packages/core/src/orphan.test.ts`

```typescript
describe("findOrphanRecords", () => {
  test("returns empty for skill with existing directory");
  test("detects directory-missing for standalone skill");
  test("detects directory-missing for disabled skill");
  test("detects cache-missing for multi-skill");
  test("detects cache-subdir-missing when cache exists but subdirectory doesn't");
  test("detects link-target-missing for linked skill");
  test("detects directory-missing for npm skill");
  test("skips active=false skills correctly (checks disabled dir, not active dir)");
  test("handles mixed orphans and healthy records");
});

describe("purgeOrphanRecords", () => {
  test("removes specified orphan records from installed.json");
  test("removes agent symlinks for purged records");
  test("returns names of purged records");
  test("saves installed.json after purging");
  test("handles empty orphan list (no-op)");
});

describe("formatOrphanReason", () => {
  test("formats all 4 reason types");
});
```

### Integration Tests: `packages/core/src/update.test.ts` (additions)

```typescript
describe("updateSkill — orphan handling", () => {
  test("detects and purges orphan records before update pass");
  test("does not crash when multi-skill subdirectory is missing from cache after pull");
  test("calls onSkillRemovedUpstream when subdirectory is missing");
  test("does not crash when standalone skill install dir is missing");
  test("reports removed-upstream progress status");
});
```

### Integration Tests: `packages/core/src/install.test.ts` (additions)

```typescript
describe("installSkill — phantom conflict handling", () => {
  test("installs successfully when conflict record exists but directory is missing");
  test("cleans up stale conflict record before proceeding with install");
  test("calls onOrphansFound when orphan records exist");
});
```

## Verification Checklist

```bash
# New tests
bun test packages/core/src/orphan.test.ts

# Regression — existing tests must pass
bun test packages/core/src/update.test.ts
bun test packages/core/src/install.test.ts
bun test packages/core/src/remove.test.ts

# Full suite
bun test

# Manual verification of the exact failure
# 1. Install a multi-skill repo
# 2. Delete a skill subdirectory from the git cache
# 3. Run `skilltap update` — should NOT crash, should offer to clean up
```
