# Design: Phase 31c-a — Manifest + Lockfile Writes from Install

## Overview

When a user installs a skill or plugin in a project that has a `skilltap.toml`, automatically append the new dependency to the manifest's `[skills]` or `[plugins]` table AND write a corresponding entry to `skilltap.lock`. If no `skilltap.toml` exists, the install is a no-op for manifest writes — purely additive, doesn't change existing behavior.

This is the smallest meaningful slice of the Phase 31c cutover. It delivers immediate user value (commit `skilltap.toml` + `skilltap.lock`, teammates see your project deps) without touching install reads or v1 retirement.

## Scope

In:
- New helper `core/src/manifest/update.ts` with `addSkillToManifest()` and `addPluginToManifest()`.
- Source-key canonicalization (`https://github.com/owner/repo[.git]` → `github:owner/repo`).
- Wire-up from `install.ts` after `saveInstalled` succeeds — for skills.
- Wire-up from plugin install after a plugin record is saved — for plugins.
- Tests for the helper + an integration test that runs install end-to-end and asserts the manifest+lockfile are updated.

Out (deferred to subsequent 31c phases):
- Manifest writes from `remove` (Phase 31c-b).
- Sync apply (Phase 31c-b).
- state.json reads cutover (Phase 31c-c).
- Smart scope default (Phase 31c-c).
- Agent flag full cutover (Phase 31c-c).
- `mcp:` install prefix (Phase 31c-c).

## Autonomous Decisions

### D1. No-op without `skilltap.toml`

If the project root doesn't contain `skilltap.toml`, `addSkillToManifest` / `addPluginToManifest` return `ok(undefined)` without doing anything. This makes the feature opt-in: users explicitly choose v2.0 manifest workflow by creating the file.

### D2. Manifest-write failures don't fail the install

If the manifest write fails (TOML serialization, disk error), log a debug warning but return success from install. The skill is already in `installed.json` — we shouldn't roll that back over a manifest hiccup. The user can re-run `install` or fix the manifest manually.

### D3. Source-key canonicalization is best-effort

Convert obvious GitHub URLs to `github:owner/repo` shorthand. Leave npm:, local paths, and unknown URLs as-is. The user can edit the manifest later if they prefer a different style. We don't try to reverse-engineer the user's exact original input — `record.repo` is what's available post-install and that's what we work with.

### D4. Range strategy for new entries

For Phase 31c-a, all new manifest entries get `"*"` as their range. Users can tighten to `"^1.0"` or specific tags by hand. Future phases (proper `install --version` support) can populate this more precisely. Lockfile gets the actual ref + sha.

### D5. Plugins use the same code path

A plugin install also writes to `skilltap.toml` (under `[plugins]`) with the plugin name as source key. Lockfile entry goes to `lockfile.plugin[]`. The `installPlugin` flow already produces a `PluginRecord` — we extract `repo` + `ref` + `sha` from there.

## Implementation Units

### Unit 1 — `core/src/manifest/update.ts`

```typescript
import { manifestExists, loadManifest } from "./load";
import { saveManifest } from "./save";
import { loadLockfile, saveLockfile } from "./lockfile";
import { type Lockfile, type LockEntry, LockfileSchema, type ManifestEntry, type ProjectManifest } from "./schemas";
import { ok, type Result, type UserError } from "../types";

export interface ManifestUpdateInput {
  /** Original source string preferred; falls back to record.repo. */
  source: string;
  /** Branch/tag/sha installed (the reference for the lockfile entry). */
  ref: string | null;
  /** Resolved sha at install time. */
  sha: string | null;
  /** Range to record in the manifest. Defaults to "*". */
  range?: string;
}

/**
 * Convert an install record's repo URL into the canonical manifest source key.
 * - `https://github.com/owner/repo[.git]` → `github:owner/repo`
 * - `git@github.com:owner/repo.git` → `github:owner/repo`
 * - `npm:@scope/name[@version]` → unchanged
 * - Everything else: passthrough.
 */
export function canonicalizeSourceKey(repoOrSource: string): string {
  // SSH form: git@host:owner/repo.git
  const sshMatch = /^git@([^:]+):([^/]+)\/([^.]+?)(?:\.git)?$/.exec(repoOrSource);
  if (sshMatch && sshMatch[1] === "github.com") {
    return `github:${sshMatch[2]}/${sshMatch[3]}`;
  }

  // HTTPS form: https://github.com/owner/repo[.git]
  const httpsMatch = /^https?:\/\/github\.com\/([^/]+)\/([^/]+?)(?:\.git)?$/.exec(repoOrSource);
  if (httpsMatch) {
    return `github:${httpsMatch[1]}/${httpsMatch[2]}`;
  }

  return repoOrSource;
}

/**
 * Append (or replace) a skill entry in skilltap.toml + skilltap.lock at the
 * project root. No-op if no manifest exists at projectRoot.
 *
 * The skill entry is added to the [skills] table with a "*" range (the user
 * can tighten later). The lockfile gets a precise { ref, sha } record.
 */
export async function addSkillToManifest(
  projectRoot: string,
  input: ManifestUpdateInput,
): Promise<Result<void, UserError>> {
  return updateManifestEntry(projectRoot, input, "skills");
}

export async function addPluginToManifest(
  projectRoot: string,
  input: ManifestUpdateInput,
): Promise<Result<void, UserError>> {
  return updateManifestEntry(projectRoot, input, "plugins");
}

async function updateManifestEntry(
  projectRoot: string,
  input: ManifestUpdateInput,
  kind: "skills" | "plugins",
): Promise<Result<void, UserError>> {
  if (!(await manifestExists(projectRoot))) return ok(undefined);

  const sourceKey = canonicalizeSourceKey(input.source);
  const range = input.range ?? "*";

  // ── Manifest update ─────────────────────────────────────────────────────
  const manifestResult = await loadManifest(projectRoot);
  if (!manifestResult.ok) return ok(undefined); // Don't fail install on parse issues
  const manifest = manifestResult.value;
  const updated: ProjectManifest = {
    ...manifest,
    [kind]: {
      ...manifest[kind],
      [sourceKey]: range as ManifestEntry,
    },
  };
  const saveManifestResult = await saveManifest(projectRoot, updated);
  if (!saveManifestResult.ok) return ok(undefined);

  // ── Lockfile update ─────────────────────────────────────────────────────
  const lockfileResult = await loadLockfile(projectRoot);
  if (!lockfileResult.ok) return ok(undefined);
  const lockfile = lockfileResult.value;

  // Replace any existing entry for this source; append if new.
  const existingIdx = lockfile[kind === "skills" ? "skill" : "plugin"].findIndex(
    (e) => e.source === sourceKey,
  );
  const newEntry: LockEntry = {
    source: sourceKey,
    ref: input.ref ?? "",
    sha: input.sha ?? undefined,
    range,
  };
  const targetArray = kind === "skills" ? "skill" : "plugin";
  const existing = lockfile[targetArray];
  const nextEntries =
    existingIdx === -1
      ? [...existing, newEntry]
      : existing.map((e, i) => (i === existingIdx ? newEntry : e));
  const nextLockfile: Lockfile = LockfileSchema.parse({
    version: 1,
    skill: targetArray === "skill" ? nextEntries : lockfile.skill,
    plugin: targetArray === "plugin" ? nextEntries : lockfile.plugin,
  });

  await saveLockfile(projectRoot, nextLockfile);
  return ok(undefined);
}
```

**Acceptance Criteria**:
- [ ] `canonicalizeSourceKey("https://github.com/n/r")` → `"github:n/r"`.
- [ ] `canonicalizeSourceKey("https://github.com/n/r.git")` → `"github:n/r"`.
- [ ] `canonicalizeSourceKey("git@github.com:n/r.git")` → `"github:n/r"`.
- [ ] `canonicalizeSourceKey("npm:@scope/x")` → `"npm:@scope/x"`.
- [ ] `canonicalizeSourceKey("https://example.com/repo.git")` → unchanged.
- [ ] `addSkillToManifest` no-op when no `skilltap.toml` at projectRoot.
- [ ] `addSkillToManifest` appends to `[skills]` table when `skilltap.toml` exists.
- [ ] Re-running `addSkillToManifest` for the same source replaces the lockfile entry (doesn't duplicate).
- [ ] Lockfile entries land in `lockfile.skill[]` (or `lockfile.plugin[]` for plugins) with correct ref+sha.
- [ ] Manifest range value defaults to `"*"`.

### Unit 2 — Wire `addSkillToManifest` into `install.ts`

After the existing `saveInstalled(installed, fileRoot)` call (around line 769), iterate `newRecords` and call `addSkillToManifest` for each, but ONLY when `options.scope === "project"` AND `options.projectRoot` is set:

```typescript
// 11. v2 manifest update (Phase 31c-a) — no-op without skilltap.toml
if (options.scope === "project" && options.projectRoot) {
  for (const record of newRecords) {
    if (!record.repo) continue;  // linked skills have no source
    await addSkillToManifest(options.projectRoot, {
      source: record.repo,
      ref: record.ref,
      sha: record.sha,
    }).catch(() => {
      // Manifest write failures are non-fatal. Logged via debug.
    });
  }
}
```

Add the import at the top of install.ts.

**Acceptance Criteria**:
- [ ] Project-scope install in a project with `skilltap.toml` → manifest+lockfile updated.
- [ ] Project-scope install in a project WITHOUT `skilltap.toml` → no manifest written; install still succeeds.
- [ ] Global-scope install → no project manifest writes (correct: global installs don't belong to a project).
- [ ] Linked skill install (no repo) → skipped silently.

### Unit 3 — Wire `addPluginToManifest` into plugin install

The plugin install flow at `core/src/plugin/install.ts` produces a `PluginRecord`. After `addPlugin(state, record)` + `savePlugins(...)` succeeds, call `addPluginToManifest(projectRoot, ...)` similarly to Unit 2.

The plugin record has `.repo`, `.ref`, `.sha` fields — same shape as `InstalledSkill`. Source key for a plugin is the repo URL (or canonicalized form).

**Acceptance Criteria**:
- [ ] Project-scope plugin install in a project with `skilltap.toml` → `[plugins]` table gets the new entry with the plugin's repo URL.
- [ ] Lockfile gets a `plugin` entry with ref + sha.
- [ ] Same no-op behavior when `skilltap.toml` is absent.

### Unit 4 — Tests

**`core/src/manifest/update.test.ts`** — covers:
- `canonicalizeSourceKey` cases (https/ssh/npm/passthrough).
- `addSkillToManifest` no-op without manifest.
- `addSkillToManifest` appends to fresh manifest (creates the [skills] entry).
- `addSkillToManifest` updates the existing entry (no duplicate).
- Lockfile is created/updated correctly.
- `addPluginToManifest` writes to [plugins] and `lockfile.plugin[]`.

These tests use temp project roots (`mkdtemp`) and synthesize `skilltap.toml` files via `writeFile`. No need for `createTestEnv` — just project-scoped tests.

## Implementation Order

1. Unit 1 (`update.ts`) — pure helper.
2. Unit 4 (tests for Unit 1) — verify before wiring.
3. Unit 2 (install.ts wire-up).
4. Unit 3 (plugin/install.ts wire-up).
5. Manual smoke: run `skilltap install` in a project with a hand-written `skilltap.toml`, verify entries appear.

## Verification

```bash
bun test packages/core/src/manifest/update.test.ts
bun test packages/core/src/manifest/

# Full v2 baseline
bun test packages/core/src/manifest/ packages/core/src/state/ packages/core/src/migrate/ packages/core/src/sync/ packages/core/src/plugin-v2/ packages/core/src/plugin/detect.test.ts packages/core/src/plugin/component-ref.test.ts packages/core/src/plugin/mcp-inject.claude-desktop.test.ts packages/core/src/schemas/config-v2.test.ts packages/core/src/policy-v2/ packages/core/src/status/ packages/core/src/try.test.ts packages/core/src/doctor/
```

## Out of Scope

- Manifest writes from `remove` — Phase 31c-b.
- Replacing v1 reads — Phase 31c-c.
- Sync apply implementation — Phase 31c-b.
- Smart scope default — Phase 31c-c.
- Agent flag cutover — Phase 31c-c.
- `mcp:` install prefix — Phase 31c-c.
- v1 schema retirement — Phase 31c-c.
