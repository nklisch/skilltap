# Design: Phase 31c-b-2 — Sync Apply

## Overview

`skilltap sync --apply` (currently errors out) actually executes the SyncPlan. For each ordered drift item, dispatch to the right handler:

- `add` (skill/plugin) → call `installSkill` with auto-accept callbacks
- `remove` (skill/plugin) → look up name from state, call `removeSkill` or `removeInstalledPlugin`
- `ref-mismatch` → install with `onAlreadyInstalled: () => "update"`
- `lock-missing`, `lock-stale`, `lock-orphan` → out of scope (informational; surfaced via `update`/`sync --prune`)

## Autonomous Decisions

### D1. Apply uses existing v1 install/remove machinery

For Phase 31c-b-2, sync apply calls the existing `installSkill` / `removeSkill` / `installPlugin` / `removeInstalledPlugin`. v1 path. After Phase 31c-c cuts over to v2 readers/writers, sync apply will continue to work because it just calls the orchestrator functions which themselves change.

### D2. Auto-accept callbacks

Apply runs without prompts. All `on*` callbacks return immediate continue:
- `onSelectSkills`: install all
- `onWarnings`/`onSemanticWarnings`/`onConfirmInstall`: return `true`
- `onAlreadyInstalled`: return `"update"` (force update for ref-mismatch case)
- `onDeepScan`: return `true`
- `onPluginDetected`: return `"plugin"` (treat as plugin if detected)

If `--strict` is set, warnings turn into hard errors per item. For Phase 31c-b-2, strict is implemented as: if any item produces a warning, the apply for that item fails with the warning. (Simpler than v1's `composePolicy` strict logic.)

### D3. Dependency injection for testability

`applySync` takes optional `installFn` / `removeFn` / `installPluginFn` / `removeInstalledPluginFn` parameters that default to the real implementations. Tests inject mocks. Matches the project's Injectable Dependencies pattern documented in `.claude/rules/patterns.md`.

### D4. Lock-* items are not auto-applied

`lock-missing`/`lock-stale`/`lock-orphan` items get reported but skipped. They're book-keeping — the user resolves via `update` (refresh lockfile) or `sync --prune` (remove orphans). Skipping is correct: applying without explicit user intent could destroy work.

### D5. Errors don't stop the apply (unless --strict)

If an item fails, log the error and continue with the rest. Final `SyncApplyResult.errors[]` lists all failures. Exit code from CLI is non-zero if any errors occurred.

With `--strict`: fail-fast on the first error.

### D6. Source string lookup for "remove"

For a `remove` drift item, the `source` field is the manifest source key (e.g., `github:n/foo`). State records have `repo` (which canonicalizes to the same key). Look up the skill name by matching `canonicalizeSourceKey(repo) === source`. If multiple state records share the source (rare — same source in different scopes), prefer the project-scope record.

## Implementation Units

### Unit 1 — `core/src/sync/apply.ts`

```typescript
import {
  type GitError,
  type NetworkError,
  type Result,
  type ScanError,
  UserError,
  err,
  ok,
} from "../types";
import { installSkill, type InstallResult } from "../install";
import { installPlugin, type PluginInstallResult } from "../plugin/install";
import { removeSkill } from "../remove";
import { removeInstalledPlugin } from "../plugin/lifecycle";
import { canonicalizeSourceKey } from "../manifest/update";
import type { State } from "../state/schema";
import type { DriftItem, SyncPlan } from "./types";

export type ApplyStatus = "ok" | "skipped" | "fail";

export interface ApplyItemResult {
  item: DriftItem;
  status: ApplyStatus;
  error?: string;
}

export interface SyncApplyResult {
  results: ApplyItemResult[];
  applied: number;
  skipped: number;
  failed: number;
}

export interface SyncApplyOptions {
  projectRoot: string;
  /** State to look up names from — passed in to avoid a re-load round-trip. */
  state: State;
  /** Stop on first failure. Default false. */
  strict?: boolean;
  /** Per-item progress callback. */
  onProgress?: (item: DriftItem, status: ApplyStatus, error?: string) => void;

  // Injectable for tests — default to real implementations
  installFn?: typeof installSkill;
  removeSkillFn?: typeof removeSkill;
  installPluginFn?: typeof installPlugin;
  removeInstalledPluginFn?: typeof removeInstalledPlugin;
}

export async function applySync(
  plan: SyncPlan,
  options: SyncApplyOptions,
): Promise<Result<SyncApplyResult, UserError>> {
  const installFn = options.installFn ?? installSkill;
  const removeSkillFn = options.removeSkillFn ?? removeSkill;
  const installPluginFn = options.installPluginFn ?? installPlugin;
  const removeInstalledPluginFn = options.removeInstalledPluginFn ?? removeInstalledPlugin;

  const results: ApplyItemResult[] = [];
  let applied = 0;
  let skipped = 0;
  let failed = 0;

  for (const item of plan.ordered) {
    const status = await applyItem(item, options, {
      installFn, removeSkillFn, installPluginFn, removeInstalledPluginFn,
    });
    results.push({ item, status: status.status, error: status.error });
    options.onProgress?.(item, status.status, status.error);

    if (status.status === "ok") applied++;
    else if (status.status === "skipped") skipped++;
    else failed++;

    if (status.status === "fail" && options.strict) {
      return ok({ results, applied, skipped, failed });
    }
  }

  return ok({ results, applied, skipped, failed });
}

interface ApplyItemFns {
  installFn: typeof installSkill;
  removeSkillFn: typeof removeSkill;
  installPluginFn: typeof installPlugin;
  removeInstalledPluginFn: typeof removeInstalledPlugin;
}

async function applyItem(
  item: DriftItem,
  options: SyncApplyOptions,
  fns: ApplyItemFns,
): Promise<{ status: ApplyStatus; error?: string }> {
  // lock-* items are informational only
  if (item.kind === "lock-missing" || item.kind === "lock-stale" || item.kind === "lock-orphan") {
    return { status: "skipped" };
  }

  if (item.target === "skill") {
    if (item.kind === "remove") {
      return await applyRemoveSkill(item, options, fns);
    }
    if (item.kind === "add" || item.kind === "ref-mismatch") {
      return await applyAddSkill(item, options, fns);
    }
  }

  if (item.target === "plugin") {
    if (item.kind === "remove") {
      return await applyRemovePlugin(item, options, fns);
    }
    if (item.kind === "add" || item.kind === "ref-mismatch") {
      return await applyAddPlugin(item, options, fns);
    }
  }

  return { status: "skipped" };
}

async function applyAddSkill(
  item: DriftItem,
  options: SyncApplyOptions,
  fns: ApplyItemFns,
): Promise<{ status: ApplyStatus; error?: string }> {
  const ref = item.declared?.ref;
  const result = await fns.installFn(item.source, {
    scope: "project",
    projectRoot: options.projectRoot,
    ref,
    onSelectSkills: async (skills) => skills.map((s) => s.name),
    onWarnings: async () => !options.strict,
    onSemanticWarnings: async () => !options.strict,
    onConfirmInstall: async () => true,
    onAlreadyInstalled: async () => "update",
    onDeepScan: async () => true,
    onPluginDetected: async () => "plugin",
    onPluginWarnings: async () => !options.strict,
    onPluginConfirm: async () => true,
  });
  if (!result.ok) return { status: "fail", error: result.error.message };
  return { status: "ok" };
}

async function applyRemoveSkill(
  item: DriftItem,
  options: SyncApplyOptions,
  fns: ApplyItemFns,
): Promise<{ status: ApplyStatus; error?: string }> {
  const name = findSkillNameBySource(options.state, item.source);
  if (!name) {
    return { status: "fail", error: `state has no skill matching source ${item.source}` };
  }
  const result = await fns.removeSkillFn(name, {
    scope: "project",
    projectRoot: options.projectRoot,
  });
  if (!result.ok) return { status: "fail", error: result.error.message };
  return { status: "ok" };
}

async function applyAddPlugin(
  item: DriftItem,
  options: SyncApplyOptions,
  fns: ApplyItemFns,
): Promise<{ status: ApplyStatus; error?: string }> {
  // Plugin install via the regular installSkill path — it auto-detects plugin.
  return applyAddSkill(item, options, fns);
}

async function applyRemovePlugin(
  item: DriftItem,
  options: SyncApplyOptions,
  fns: ApplyItemFns,
): Promise<{ status: ApplyStatus; error?: string }> {
  const name = findPluginNameBySource(options.state, item.source);
  if (!name) {
    return { status: "fail", error: `state has no plugin matching source ${item.source}` };
  }
  const result = await fns.removeInstalledPluginFn(name, {
    scope: "project",
    projectRoot: options.projectRoot,
  });
  if (!result.ok) return { status: "fail", error: result.error.message };
  return { status: "ok" };
}

function findSkillNameBySource(state: State, source: string): string | null {
  for (const s of state.skills) {
    if (s.repo && canonicalizeSourceKey(s.repo) === source) return s.name;
  }
  return null;
}

function findPluginNameBySource(state: State, source: string): string | null {
  for (const p of state.plugins) {
    if (p.repo && canonicalizeSourceKey(p.repo) === source) return p.name;
  }
  return null;
}
```

**Acceptance Criteria**:
- [ ] In-sync plan applies cleanly with applied=0/skipped=0/failed=0.
- [ ] Add-only plan calls installFn for each add item; counts applied++.
- [ ] Remove-only plan calls removeSkillFn for each remove item; counts applied++.
- [ ] Mixed plan respects ordering (remove → ref-mismatch → add).
- [ ] lock-* items always count as skipped.
- [ ] Failure in non-strict mode continues with subsequent items.
- [ ] Failure in strict mode stops at the first failure.
- [ ] `applyRemoveSkill` failures when state lacks a record matching the source.

### Unit 2 — Update `core/src/sync/index.ts`

Add `export * from "./apply"`.

### Unit 3 — Update `cli/src/commands/sync.ts`

Replace the `--apply` error path with real execution. Load state via `loadState(projectRoot)`, build the plan, then call `applySync` with `--strict` if requested. Print per-item progress and a final summary. Exit 1 if any items failed.

### Unit 4 — Tests

`packages/core/src/sync/apply.test.ts` — covers all dispatch paths via injected mock functions. No network; no real installs.

## Implementation Order

1. Unit 1 (`apply.ts`) — pure module.
2. Unit 4 (tests) — verify each path before wiring.
3. Unit 2 (barrel export).
4. Unit 3 (CLI wire-up).

## Verification

```bash
bun test packages/core/src/sync/

# Smoke: sync --apply on a clean project (no drift) succeeds with no-op
SKILLTAP_NO_STARTUP=1 SKILLTAP_HOME=/tmp/.t bun packages/cli/src/index.ts sync --apply

# Full v2 baseline still passes
bun test packages/core/src/manifest/ packages/core/src/state/ packages/core/src/migrate/ packages/core/src/sync/ packages/core/src/plugin-v2/ packages/core/src/plugin/detect.test.ts packages/core/src/plugin/component-ref.test.ts packages/core/src/plugin/mcp-inject.claude-desktop.test.ts packages/core/src/schemas/config-v2.test.ts packages/core/src/policy-v2/ packages/core/src/status/ packages/core/src/try.test.ts packages/core/src/doctor/
```

## Out of Scope

- Lockfile updates after apply — defer to Phase 31c-c (where state.json reads happen and lockfile naturally syncs).
- `--prune` flag for orphan removal — Phase 31c-c.
- Smart scope default — Phase 31c-c.
- Component-level toggle synchronization — out of v2.0 scope.
