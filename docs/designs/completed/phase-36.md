# Design: Phase 36 — Doctor v2.0 Upgrades

## Overview

Add four new checks to `skilltap doctor` that surface v2.0-specific drift and inconsistency, plus extend `--fix` for the safely-fixable subset. Existing 9 checks remain unchanged.

New checks (orchestration order):

1. `state` — load `state.json` (v2). Emits "no v2 state" detail if the file is absent (pre-migration users); fixable corruption recovery.
2. `manifest-drift` — compares `skilltap.toml` against state via existing `detectDrift`. Surfaces declared-vs-installed mismatches. Not auto-fixable (we don't edit user manifests).
3. `lockfile-drift` — compares `skilltap.lock` against state. Auto-fixable: regenerate missing lockfile entries from state. Manual: lockfile-stale-sha and lockfile-orphan are surfaced as warnings.
4. `plugin-manifests` — walks `.skilltap/*.toml` (publish manifests in current cwd), reports parse errors and schema mismatches.
5. `mcp-consistency` — bidirectional. State-active MCP servers should be present in agent config files; agent config entries with `skilltap:` prefix should have corresponding state records. Orphan agent-config entries are auto-fixable (prune).

## Autonomous Decisions

### D1. Coexist with v1 checks (don't remove yet)

Existing `installed`/`skills`/`symlinks` checks keep reading `installed.json`. Phase 36 adds v2 checks alongside. Phase 31c's cutover is the right time to retire v1 checks.

Rationale: a doctor that reports both v1 and v2 state during the transition is more useful than one that flips overnight. Pre-migration users see v1 + "no v2 state"; post-migration users see v1-empty + populated v2.

### D2. State load chain pattern

Mirror the `checkInstalled` shape: `checkStateV2(projectRoot?)` returns `{ check: DoctorCheck, state: State | null }`. Downstream v2 checks accept state as a parameter and emit a "n/a (no v2 state)" detail if it's null. This keeps each new check pure and gives `runDoctor` one place to handle the load.

### D3. Manifest drift discovery uses cwd, not project root

`gatherStatus` from Phase 33a uses `tryFindProjectRoot()` (walks up looking for `.git`). For doctor, follow the same pattern — use the passed `projectRoot` or cwd. If no manifest at the root, emit "no manifest" and skip drift work (no warning — many users don't use the manifest).

### D4. Plugin-manifests check scope = current repo

The `.skilltap/<plugin>.toml` files are publish manifests for THE CURRENT REPO (whoever is running doctor). Walk `<projectRoot>/.skilltap/*.toml`. If `.skilltap/` doesn't exist, emit "n/a (no publish manifests)". If files exist but parse poorly, warn per file.

### D5. MCP consistency: only check active components

A plugin's MCP component is active iff its `active` field is true AND the plugin itself is active. Inactive components don't need agent-config presence — they were intentionally toggled off. Skip them.

### D6. MCP orphan auto-fix

An "orphan" agent-config MCP entry (key starts with `skilltap:`, but no matching state record) gets pruned by --fix. Use the existing `removeMcpServers` helper from `mcp-inject.ts` — it knows how to locate and remove namespaced entries. Pass `pluginName` parsed from the namespaced key (`skilltap:<plugin>:<server>` → pluginName).

### D7. Fixable lockfile entries

When state has a skill/plugin record with `repo` and `sha`, but `lockfile.skill[]` / `lockfile.plugin[]` lacks an entry for that source, --fix appends a lock entry: `{ source: state.repo, ref: state.ref, sha: state.sha, range: state.ref ?? "*" }`. The range value is approximate — it's the recorded ref, not what the user originally declared. Mark fix description as "regenerated from state".

For lockfile-stale-sha (locked sha ≠ installed sha) → not fixable. User runs `skilltap update` or `skilltap sync`.
For lockfile-orphan (locked entry, no state, no manifest) → not fixable. User runs `skilltap sync --prune`.

## Implementation Units

### Unit 1 — `core/src/doctor/checks/state-v2.ts`

```typescript
import { copyFile, writeFile } from "node:fs/promises";
import { z } from "zod/v4";
import { fileExists } from "../../fs";
import { type State, StateSchema } from "../../state/schema";
import { getStatePath } from "../../state/paths";
import type { DoctorCheck, DoctorIssue } from "../types";

const DEFAULT_STATE: State = { version: 2, skills: [], plugins: [], mcpServers: [] };

async function readStateFile(
  file: string,
  label: string,
  issues: DoctorIssue[],
): Promise<State | null> {
  if (!(await fileExists(file))) return null;
  let raw: unknown;
  try {
    raw = await Bun.file(file).json();
  } catch (e) {
    issues.push({
      message: `${label} is corrupt: ${e}`,
      fixable: true,
      fixDescription: `backed up to ${label}.bak, created fresh`,
      fix: async () => {
        await copyFile(file, `${file}.bak`).catch(() => {});
        await writeFile(file, JSON.stringify(DEFAULT_STATE, null, 2));
      },
    });
    return null;
  }
  const result = StateSchema.safeParse(raw);
  if (!result.success) {
    issues.push({
      message: `${label} is invalid: ${z.prettifyError(result.error)}`,
      fixable: true,
      fixDescription: `backed up to ${label}.bak, created fresh`,
      fix: async () => {
        await copyFile(file, `${file}.bak`).catch(() => {});
        await writeFile(file, JSON.stringify(DEFAULT_STATE, null, 2));
      },
    });
    return null;
  }
  return result.data;
}

export async function checkStateV2(projectRoot?: string): Promise<{
  check: DoctorCheck;
  state: State | null;
}> {
  const issues: DoctorIssue[] = [];
  const globalFile = getStatePath();
  const projectFile = projectRoot ? getStatePath(projectRoot) : null;

  const globalState = await readStateFile(globalFile, "state.json", issues);
  const projectState = projectFile
    ? await readStateFile(projectFile, ".agents/state.json", issues)
    : null;

  const merged: State | null = globalState || projectState
    ? {
        version: 2,
        skills: [...(globalState?.skills ?? []), ...(projectState?.skills ?? [])],
        plugins: [...(globalState?.plugins ?? []), ...(projectState?.plugins ?? [])],
        mcpServers: [
          ...(globalState?.mcpServers ?? []),
          ...(projectState?.mcpServers ?? []),
        ],
      }
    : null;

  if (issues.length > 0) {
    return { check: { name: "state.json", status: "fail", issues }, state: merged };
  }

  if (!merged) {
    return {
      check: {
        name: "state.json",
        status: "pass",
        detail: "n/a (no v2 state — run 'skilltap migrate' to upgrade)",
      },
      state: null,
    };
  }

  const skillCount = merged.skills.length;
  const pluginCount = merged.plugins.length;
  const mcpCount = merged.mcpServers.length;
  const detail = `${skillCount} skill${skillCount === 1 ? "" : "s"}, ${pluginCount} plugin${pluginCount === 1 ? "" : "s"}, ${mcpCount} standalone MCP${mcpCount === 1 ? "" : "s"}`;
  return { check: { name: "state.json", status: "pass", detail }, state: merged };
}
```

**Acceptance Criteria**:
- [ ] When state.json absent: pass with "n/a (no v2 state...)" detail.
- [ ] When state.json present and valid: pass with skill/plugin/MCP count detail.
- [ ] When state.json corrupt JSON: fail with fixable issue (backup + recreate).
- [ ] When state.json valid JSON but invalid schema: fail with fixable issue.
- [ ] Project-scope state.json read alongside global; merged for downstream consumers.

---

### Unit 2 — `core/src/doctor/checks/manifest-drift.ts`

```typescript
import { type Lockfile, LockfileSchema } from "../../manifest/schemas";
import { loadLockfile, loadManifest, manifestExists } from "../../manifest";
import { detectDrift } from "../../sync/drift";
import type { State } from "../../state/schema";
import type { DoctorCheck, DoctorIssue } from "../types";

const EMPTY_LOCKFILE: Lockfile = LockfileSchema.parse({ version: 1 });

export async function checkManifestDrift(
  state: State | null,
  projectRoot?: string,
): Promise<DoctorCheck> {
  if (!state) {
    return {
      name: "manifest drift",
      status: "pass",
      detail: "n/a (no v2 state)",
    };
  }
  if (!projectRoot) {
    return {
      name: "manifest drift",
      status: "pass",
      detail: "n/a (no project root)",
    };
  }
  if (!(await manifestExists(projectRoot))) {
    return {
      name: "manifest drift",
      status: "pass",
      detail: "n/a (no skilltap.toml)",
    };
  }

  const manifestResult = await loadManifest(projectRoot);
  if (!manifestResult.ok) {
    return {
      name: "manifest drift",
      status: "fail",
      issues: [{ message: `Failed to load manifest: ${manifestResult.error.message}`, fixable: false }],
    };
  }
  const lockfileResult = await loadLockfile(projectRoot);
  const lockfile = lockfileResult.ok ? lockfileResult.value : EMPTY_LOCKFILE;

  const drift = detectDrift(manifestResult.value, lockfile, state);

  if (drift.inSync) {
    return {
      name: "manifest drift",
      status: "pass",
      detail: "in sync",
    };
  }

  // Surface only manifest-vs-state drift here (add/remove/ref-mismatch).
  // Lockfile-specific items (lock-missing/lock-stale/lock-orphan) are
  // owned by the lockfile-drift check.
  const items = drift.items.filter(
    (i) => i.kind === "add" || i.kind === "remove" || i.kind === "ref-mismatch",
  );
  if (items.length === 0) {
    return {
      name: "manifest drift",
      status: "pass",
      detail: "in sync",
    };
  }

  const issues: DoctorIssue[] = items.map((item) => ({
    message: `${item.kind}: ${item.target} ${item.source}${item.reason ? ` — ${item.reason}` : ""}`,
    fixable: false,
  }));

  return {
    name: "manifest drift",
    status: "warn",
    detail: `${items.length} drift item${items.length === 1 ? "" : "s"} — run 'skilltap sync' for details`,
    issues,
  };
}
```

**Acceptance Criteria**:
- [ ] No state → pass "n/a (no v2 state)".
- [ ] No projectRoot → pass "n/a (no project root)".
- [ ] No skilltap.toml → pass "n/a (no skilltap.toml)".
- [ ] Manifest matches state → pass "in sync".
- [ ] Manifest declares an entry that's not in state → warn with one issue per drift item.
- [ ] Issues are not fixable (we don't auto-edit manifests).

---

### Unit 3 — `core/src/doctor/checks/lockfile-drift.ts`

```typescript
import { type Lockfile, type LockEntry, LockfileSchema, loadLockfile, lockfileExists, saveLockfile } from "../../manifest";
import { type State } from "../../state/schema";
import type { DoctorCheck, DoctorIssue } from "../types";

export async function checkLockfileDrift(
  state: State | null,
  projectRoot?: string,
): Promise<DoctorCheck> {
  if (!state || !projectRoot) {
    return {
      name: "lockfile drift",
      status: "pass",
      detail: !state ? "n/a (no v2 state)" : "n/a (no project root)",
    };
  }
  if (!(await lockfileExists(projectRoot))) {
    return {
      name: "lockfile drift",
      status: "pass",
      detail: "n/a (no skilltap.lock)",
    };
  }
  const result = await loadLockfile(projectRoot);
  if (!result.ok) {
    return {
      name: "lockfile drift",
      status: "fail",
      issues: [{ message: result.error.message, fixable: false }],
    };
  }
  const lockfile = result.value;

  const issues: DoctorIssue[] = [];

  // Build set of source strings the state knows about
  const stateSourceMap = new Map<
    string,
    { kind: "skill" | "plugin"; ref: string | null; sha: string | null }
  >();
  for (const skill of state.skills) {
    if (skill.repo) {
      stateSourceMap.set(skill.repo, { kind: "skill", ref: skill.ref, sha: skill.sha });
    }
  }
  for (const plugin of state.plugins) {
    if (plugin.repo) {
      stateSourceMap.set(plugin.repo, { kind: "plugin", ref: plugin.ref, sha: plugin.sha });
    }
  }

  const lockedSources = new Set<string>();
  for (const entry of lockfile.skill) lockedSources.add(entry.source);
  for (const entry of lockfile.plugin) lockedSources.add(entry.source);

  // 1) state-but-no-lock: fixable (append entry)
  for (const [source, info] of stateSourceMap) {
    if (!lockedSources.has(source) && info.ref) {
      issues.push({
        message: `${info.kind} '${source}' installed but missing from lockfile`,
        fixable: true,
        fixDescription: "regenerated lockfile entry from state",
        fix: async () => {
          const updated = await loadLockfile(projectRoot);
          if (!updated.ok) return;
          const newEntry: LockEntry = {
            source,
            ref: info.ref ?? "",
            sha: info.sha ?? undefined,
            range: info.ref ?? "*",
          };
          const next: Lockfile = LockfileSchema.parse({
            version: 1,
            skill: info.kind === "skill" ? [...updated.value.skill, newEntry] : updated.value.skill,
            plugin: info.kind === "plugin" ? [...updated.value.plugin, newEntry] : updated.value.plugin,
          });
          await saveLockfile(projectRoot, next);
        },
      });
    }
  }

  // 2) lockfile-stale-sha: warn, not fixable
  for (const entry of [...lockfile.skill, ...lockfile.plugin]) {
    const installed = stateSourceMap.get(entry.source);
    if (installed && entry.sha && installed.sha && entry.sha !== installed.sha) {
      issues.push({
        message: `${entry.source}: lockfile sha ${entry.sha.slice(0, 7)} differs from installed sha ${installed.sha.slice(0, 7)}`,
        fixable: false,
      });
    }
  }

  // 3) lockfile-orphan: warn, not fixable (sync --prune is the user-action)
  for (const entry of [...lockfile.skill, ...lockfile.plugin]) {
    if (!stateSourceMap.has(entry.source)) {
      issues.push({
        message: `${entry.source}: lockfile entry has no installed state`,
        fixable: false,
      });
    }
  }

  if (issues.length === 0) {
    return { name: "lockfile drift", status: "pass", detail: "in sync" };
  }
  const fixableCount = issues.filter((i) => i.fixable).length;
  return {
    name: "lockfile drift",
    status: "warn",
    detail: `${issues.length} drift item${issues.length === 1 ? "" : "s"} (${fixableCount} fixable)`,
    issues,
  };
}
```

**Acceptance Criteria**:
- [ ] No state or no projectRoot or no skilltap.lock → pass with "n/a (...)".
- [ ] state has a skill, lockfile lacks an entry for it → warn with fixable issue; --fix appends the lock entry.
- [ ] state's installed sha differs from lockfile sha → warn (not fixable).
- [ ] Lockfile entry without matching state record → warn (not fixable, points at `sync --prune`).
- [ ] In-sync → pass "in sync".

---

### Unit 4 — `core/src/doctor/checks/plugin-manifests.ts`

```typescript
import { discoverPublishablePlugins } from "../../manifest/publish";
import { publishDir } from "../../manifest/paths";
import { fileExists } from "../../fs";
import type { DoctorCheck, DoctorIssue } from "../types";

export async function checkPluginManifests(projectRoot?: string): Promise<DoctorCheck> {
  if (!projectRoot) {
    return {
      name: "plugin manifests",
      status: "pass",
      detail: "n/a (no project root)",
    };
  }
  const dir = publishDir(projectRoot);
  if (!(await fileExists(dir).catch(() => false))) {
    // Note: fileExists checks file, not dir. Try via discover (returns empty for missing dir).
  }

  const result = await discoverPublishablePlugins(projectRoot);

  const issues: DoctorIssue[] = result.rejected
    .filter((r) => !r.reason.startsWith("publish = false")) // publish=false is intentional, not an issue
    .map((r) => ({
      message: `${r.path}: ${r.reason}`,
      fixable: false,
    }));

  if (result.publishable.length === 0 && issues.length === 0) {
    return {
      name: "plugin manifests",
      status: "pass",
      detail: "n/a (no .skilltap/ publish manifests)",
    };
  }

  if (issues.length === 0) {
    return {
      name: "plugin manifests",
      status: "pass",
      detail: `${result.publishable.length} valid`,
    };
  }
  return {
    name: "plugin manifests",
    status: "warn",
    detail: `${result.publishable.length} valid, ${issues.length} invalid`,
    issues,
  };
}
```

**Acceptance Criteria**:
- [ ] No projectRoot → pass "n/a (no project root)".
- [ ] No `.skilltap/` dir → pass "n/a (no .skilltap/ publish manifests)".
- [ ] All `.skilltap/*.toml` parse with `publish = true` → pass with count.
- [ ] `.skilltap/*.toml` with parse errors or schema mismatches → warn, one issue per file.
- [ ] `publish = false` files are NOT flagged as issues (they're intentionally private).

---

### Unit 5 — `core/src/doctor/checks/mcp-consistency.ts`

```typescript
import { readFile } from "node:fs/promises";
import { fileExists } from "../../fs";
import {
  isNamespacedKey,
  MCP_AGENT_CONFIGS,
  mcpConfigPath,
  parseNamespacedKey,
  removeMcpServers,
} from "../../plugin/mcp-inject";
import type { State } from "../../state/schema";
import type { DoctorCheck, DoctorIssue } from "../types";

interface ExpectedEntry {
  pluginName: string;
  serverName: string;
  agent: string;
  scope: "global" | "project";
}

async function readMcpServersFromConfig(
  agent: string,
  scope: "global" | "project",
  projectRoot?: string,
): Promise<Set<string>> {
  const path = mcpConfigPath(agent, scope, projectRoot);
  if (!path) return new Set();
  if (!(await fileExists(path))) return new Set();
  let text: string;
  try {
    text = await readFile(path, "utf8");
  } catch {
    return new Set();
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return new Set();
  }
  if (typeof parsed !== "object" || parsed === null) return new Set();
  const obj = parsed as Record<string, unknown>;
  const servers = obj.mcpServers;
  if (typeof servers !== "object" || servers === null) return new Set();
  return new Set(Object.keys(servers as Record<string, unknown>));
}

export async function checkMcpConsistency(
  state: State | null,
  projectRoot?: string,
): Promise<DoctorCheck> {
  if (!state) {
    return {
      name: "mcp consistency",
      status: "pass",
      detail: "n/a (no v2 state)",
    };
  }

  const expected: ExpectedEntry[] = [];
  for (const plugin of state.plugins) {
    if (!plugin.active) continue;
    for (const c of plugin.components) {
      if (c.type !== "mcp" || !c.active) continue;
      for (const agent of plugin.also) {
        expected.push({
          pluginName: plugin.name,
          serverName: c.name,
          agent,
          scope: plugin.scope,
        });
      }
    }
  }

  const issues: DoctorIssue[] = [];

  // Dimension 1: state-active → must be present in agent config
  for (const e of expected) {
    const present = await readMcpServersFromConfig(e.agent, e.scope, projectRoot);
    const expectedKey = `skilltap:${e.pluginName}:${e.serverName}`;
    if (!present.has(expectedKey)) {
      issues.push({
        message: `Missing in ${e.agent} (${e.scope}): ${expectedKey}`,
        fixable: false,
      });
    }
  }

  // Dimension 2: agent-config skilltap: keys → must have a state record
  // Build a set of all expected keys (regardless of agent) to check against orphans
  const expectedKeySet = new Set(expected.map((e) => `skilltap:${e.pluginName}:${e.serverName}`));

  for (const agent of Object.keys(MCP_AGENT_CONFIGS)) {
    for (const scope of ["global", "project"] as const) {
      if (scope === "project" && !projectRoot) continue;
      const present = await readMcpServersFromConfig(agent, scope, projectRoot);
      for (const key of present) {
        if (!isNamespacedKey(key)) continue;
        if (expectedKeySet.has(key)) continue;
        const parsed = parseNamespacedKey(key);
        if (!parsed) continue;
        // Orphan — fixable via removeMcpServers
        issues.push({
          message: `Orphan in ${agent} (${scope}): ${key}`,
          fixable: true,
          fixDescription: `removed orphan from ${agent} config`,
          fix: async () => {
            await removeMcpServers({
              pluginName: parsed.pluginName,
              agents: [agent],
              scope,
              projectRoot,
            });
          },
        });
      }
    }
  }

  if (issues.length === 0) {
    return {
      name: "mcp consistency",
      status: "pass",
      detail: expected.length === 0
        ? "n/a (no active MCP servers in state)"
        : `${expected.length} server entries verified`,
    };
  }
  const fixable = issues.filter((i) => i.fixable).length;
  return {
    name: "mcp consistency",
    status: "warn",
    detail: `${issues.length} inconsistenc${issues.length === 1 ? "y" : "ies"} (${fixable} fixable)`,
    issues,
  };
}
```

**Acceptance Criteria**:
- [ ] No state → pass "n/a (no v2 state)".
- [ ] State has no active MCP components → pass "n/a (no active MCP servers in state)".
- [ ] State expects an MCP entry that's missing in agent config → warn (not fixable; user re-runs install or sync).
- [ ] Agent config has a `skilltap:foo:bar` key with no matching state record → warn, fixable; --fix calls `removeMcpServers` to prune.
- [ ] Skips inactive plugins and inactive components.

---

### Unit 6 — Wire new checks into `core/src/doctor/index.ts`

```typescript
// Adds after the existing 9-check pipeline:
const { check: stateCheck, state } = await checkStateV2(projectRoot);
await emit(stateCheck);

await emit(await checkManifestDrift(state, projectRoot));
await emit(await checkLockfileDrift(state, projectRoot));
await emit(await checkPluginManifests(projectRoot));
await emit(await checkMcpConsistency(state, projectRoot));
```

These run AFTER the existing checks so the v1 picture lands first in the output. State is loaded once; all v2 checks consume it.

**Acceptance Criteria**:
- [ ] Existing 9 checks run unchanged and in unchanged order.
- [ ] 4 new checks appended in the order documented above.
- [ ] `--fix` propagates to fixable items in the new checks.
- [ ] `result.ok = false` only when at least one check has status `"fail"` (warn doesn't break ok).

---

### Unit 7 — Tests

Spread across `packages/core/src/doctor/checks/`:

**`state-v2.test.ts`** — covers no-file, valid file, corrupt JSON (fixable), invalid schema (fixable), global+project merge.

**`manifest-drift.test.ts`** — uses `createTestEnv` + temp project root. Cases: no state → n/a, no manifest → n/a, in-sync → pass, declared-not-installed → warn.

**`lockfile-drift.test.ts`** — tmp project root with synthesized state.json + skilltap.lock. Cases: state-but-no-lock-entry (fixable), lockfile-stale-sha (warn), lockfile-orphan (warn), in-sync.

**`plugin-manifests.test.ts`** — tmp repo root with synthesized `.skilltap/*.toml` files. Cases: missing dir, valid manifests, invalid TOML, schema mismatch, publish=false (excluded).

**`mcp-consistency.test.ts`** — synthesize state.json with active MCP plugin; synthesize agent config files in tmp env. Cases: matching → pass, missing-in-config → warn, orphan-in-config → warn-fixable.

Each test file follows the existing pattern (createTestEnv, beforeEach/afterEach, Result-aware assertions).

---

## Implementation Order

1. **Unit 1** (state-v2 check) — independent, foundation for all v2 checks.
2. **Unit 2** (manifest-drift) — uses Unit 1's state.
3. **Unit 3** (lockfile-drift) — uses Unit 1's state.
4. **Unit 4** (plugin-manifests) — independent.
5. **Unit 5** (mcp-consistency) — uses Unit 1's state, plus existing mcp-inject helpers.
6. **Unit 6** (orchestrator wire-up) — after all checks exist.
7. **Unit 7** (tests) — alongside or after each check.

## Verification

```bash
bun test packages/core/src/doctor/

# Smoke check: doctor still runs end-to-end in a clean env
SKILLTAP_NO_STARTUP=1 SKILLTAP_HOME=/tmp/.t XDG_CONFIG_HOME=/tmp/.cfg bun packages/cli/src/index.ts doctor

# Full v2 baseline
bun test packages/core/src/manifest/ packages/core/src/state/ packages/core/src/migrate/ packages/core/src/sync/ packages/core/src/plugin-v2/ packages/core/src/plugin/detect.test.ts packages/core/src/plugin/component-ref.test.ts packages/core/src/plugin/mcp-inject.claude-desktop.test.ts packages/core/src/schemas/config-v2.test.ts packages/core/src/policy-v2/ packages/core/src/status/ packages/core/src/try.test.ts
```

Existing v1 doctor tests (`packages/core/src/doctor.test.ts` if present) must continue to pass.

## Out of Scope

- Removing v1 checks (`installed`/`skills`/`symlinks`) — Phase 31c cutover.
- Standalone-MCP `mcpServers[]` array consistency — empty until Phase 35b lands; defer the dedicated check.
- Cross-tap consistency (e.g., is the tap referenced in state.json present in config.taps?) — covered by existing tap check.
- Auto-fixing manifest drift by editing skilltap.toml — out by D1 (we don't edit user manifests).
- Auto-fixing lockfile-stale-sha — user runs `skilltap update` or `skilltap sync`.
