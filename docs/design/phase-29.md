# Phase 29 — Sync Engine + Command

## Goal

`skilltap sync` reports drift between `skilltap.toml`, `skilltap.lock`, and `state.json`, and prints an actionable plan. Drift detection and plan generation are first-class; status (Phase 33) and doctor (Phase 36) consume the same APIs.

## Decisions

### Defer apply to a later phase (deviation from ROADMAP 29.3 / 29.4 partial)

The roadmap calls for `sync/apply.ts` and CLI flags `--strict`, `--yes`, `--prune` that actually mutate state. v1.0 install/update/remove still write `installed.json` and `plugins.json`. Bridging from a v2 plan to v1 writers (and then re-syncing back to `state.json` after each step) doubles the writes for a transitional benefit, and the bridge will be torn out in Phase 31 when v1 readers are cut over.

Decision: Phase 29 ships drift + plan + **preview-only** sync command. The CLI prints the plan in human or JSON form; `--apply` is reserved but errors with `apply will land in Phase 31 when v1.0 readers are removed`. Status (Phase 33) and doctor (Phase 36) consume `planSync()` directly without needing apply.

This narrows Phase 29 from ~12 units to ~7 and lets us reach the front-of-house features (status dashboard, doctor drift checks) sooner — both depend on plan output, not apply.

### Drift definitions

A `DriftItem` describes one disagreement among the three sources of truth (manifest, lockfile, state). Categories:

| Kind | When |
|---|---|
| `add` | declared in manifest, not in state. (Lockfile entry may or may not exist.) |
| `remove` | in state, not declared in manifest. |
| `ref-mismatch` | declared in manifest, in state, but with a different ref/range than locked. |
| `lock-stale` | locked SHA differs from installed SHA in state. |
| `lock-missing` | declared in manifest but no lockfile entry. |
| `lock-orphan` | lockfile entry exists but neither manifest nor state references it. |

Each kind also tracks `target: "skill" | "plugin"` so consumers can group by table.

### Plan ordering

Plan items execute in this order to keep state coherent:

1. `remove` — remove undeclared things first (frees names, avoids conflicts).
2. `ref-mismatch` — change refs (treated as remove + reinstall by apply).
3. `add` — install new things.
4. `lock-stale`, `lock-missing`, `lock-orphan` — book-keeping items the user resolves with `update` (refresh lockfile) or `prune`.

Preview output groups by category for readability.

## Implementation Units

### Unit 1 — `core/src/sync/types.ts`

```typescript
export type DriftKind =
  | "add"
  | "remove"
  | "ref-mismatch"
  | "lock-stale"
  | "lock-missing"
  | "lock-orphan";

export type DriftTarget = "skill" | "plugin";

export interface DriftItem {
  kind: DriftKind;
  target: DriftTarget;
  source: string;                    // e.g. "github:n/commit-helper"
  declared?: { ref?: string; range?: string };
  installed?: { ref?: string; sha?: string };
  locked?: { ref?: string; sha?: string; range?: string };
  reason?: string;                   // optional human description
}

export interface DriftReport {
  items: DriftItem[];
  inSync: boolean;                   // true iff items.length === 0
}

export interface SyncPlan extends DriftReport {
  // Same as DriftReport — for now plan is just an ordered drift list.
  // Phase 29b/31 will extend with per-item action descriptors.
  ordered: DriftItem[];
}
```

### Unit 2 — `core/src/sync/drift.ts`

```typescript
export function detectDrift(
  manifest: ProjectManifest,
  lockfile: Lockfile,
  state: State,
): DriftReport
```

Pure function. Builds three lookup maps (manifest by source, lockfile by source, state by source) and emits one DriftItem per disagreement.

Logic per category:

- For each entry in `manifest.skills`: lookup state, lockfile.
  - state present + lockfile present + same ref → no drift.
  - state present + lockfile present + different ref → `ref-mismatch`.
  - state present + lockfile missing → `lock-missing`.
  - state absent → `add`.
- Similarly for `manifest.plugins`.
- For each entry in `state.skills` not in manifest → `remove` (target=skill).
- For each entry in `state.plugins` not in manifest → `remove` (target=plugin).
- For each entry in `lockfile.skill`/`lockfile.plugin` not referenced by manifest or state → `lock-orphan`.
- `lock-stale` is when state shows an installed sha that differs from the locked sha — same source, different sha. (Manifest declared, lockfile declares X, state shows Y.)

State entries reference their source via a normalized field. Matching from manifest to state currently uses repo URL + tap-name shorthand. For the first cut, drift compares manifest keys (which are always source identifiers) against `state.skills[].repo` / `state.plugins[].repo`. If a state record has no source string, it's treated as not-comparable and ignored (e.g., linked skills are excluded from drift).

### Unit 3 — `core/src/sync/plan.ts`

```typescript
export function planSync(report: DriftReport): SyncPlan {
  const order: DriftKind[] = [
    "remove",
    "ref-mismatch",
    "add",
    "lock-stale",
    "lock-missing",
    "lock-orphan",
  ];
  return {
    ...report,
    ordered: [...report.items].sort(
      (a, b) => order.indexOf(a.kind) - order.indexOf(b.kind),
    ),
  };
}
```

### Unit 4 — `core/src/sync/index.ts`

Barrel: re-export types, drift, plan.

### Unit 5 — `cli/src/commands/sync.ts`

Citty command. Loads manifest + lockfile + state (default project root if available, error if no project), runs detect/plan, renders a textual plan or JSON. `--json` for machine output. `--apply` reserved with a clear "not yet implemented" error pointing at PROGRESS.md.

### Unit 6 — Tests

- `sync/drift.test.ts` — every kind covered with synthetic inputs.
- `sync/plan.test.ts` — ordering and round-trip.
- (CLI tests for `sync` deferred — apply not implemented; preview is mostly text formatting which other commands cover by example.)

## Verification

```bash
bun test packages/core/src/sync/
```

Manually:

```bash
# (After phase 28 plumbing is done) place a manifest, no installs, and watch
# `skilltap sync` report N add items.
echo '[skills]\n"github:n/commit-helper" = "*"' > skilltap.toml
bun run dev sync
```

## Out of Scope

- Apply path — Phase 31 once v1 readers are removed.
- `--prune`, `--yes`, `--strict` flags for apply — Phase 31.
- Lockfile resolution on `update` — Phase 31 alongside apply.
- Source adapter dispatch (resolving manifest keys to clone URLs) — sync only compares strings; resolution belongs to apply.
