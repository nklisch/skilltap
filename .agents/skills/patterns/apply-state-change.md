# Pattern: Apply State Change

`applySkillStateChange()` provides an atomic load→mutate→save lifecycle for the skills slice of `state.json`, with optional manifest sync hooks that fire on added/removed records.

## Rationale

Direct `loadSkillState` + `saveSkillState` calls scattered across adopt, remove, and disable would each need to replicate the before/after diffing required for manifest sync. `applySkillStateChange` centralizes that logic: it loads, calls a `mutate()` function, saves, then diffs before/after by name to invoke `onAdded`/`onRemoved` hooks — guaranteeing the hooks only fire when records actually change.

## Examples

### Example 1: applySkillStateChange signature
**File**: `packages/core/src/state/apply.ts:6`
```typescript
export interface ApplyChangeOptions {
  projectRoot?: string;
  scope: "global" | "project";
  /** Returns the new array. Return null to abort without saving. */
  mutate: (current: InstalledSkill[]) => InstalledSkill[] | null;
  manifestSync?: {
    onAdded?: (record: InstalledSkill, projectRoot: string) => Promise<void>;
    onRemoved?: (record: InstalledSkill, projectRoot: string) => Promise<void>;
  };
}
```
- Returning `null` from `mutate` aborts — saves nothing, returns `ok(before)`
- `manifestSync` hooks only fire when `projectRoot` is set

### Example 2: Remove a skill record
**File**: `packages/core/src/remove.ts:95`
```typescript
const applyResult = await applySkillStateChange({
  scope,
  projectRoot,
  mutate: (current) => current.filter((r) => r.name !== name),
  manifestSync: {
    onRemoved: async (record, root) => removeSkillFromManifest(record, root),
  },
});
if (!applyResult.ok) return applyResult;
```

### Example 3: Adopt — add a new record
**File**: `packages/core/src/adopt.ts:244`
```typescript
const applyResult = await applySkillStateChange({
  scope,
  projectRoot,
  mutate: (current) => {
    const existing = current.findIndex((r) => r.name === skill.name);
    if (existing >= 0) {
      const next = [...current];
      next[existing] = newRecord;
      return next;
    }
    return [...current, newRecord];
  },
  manifestSync: {
    onAdded: async (record, root) => syncAdoptToManifest(record, root),
  },
});
if (!applyResult.ok) return applyResult;
```

### Example 4: Disable — mutate without manifest sync
**File**: `packages/core/src/disable.ts:94`
```typescript
const applyResult = await applySkillStateChange({
  scope,
  projectRoot,
  mutate: (current) =>
    current.map((r) =>
      r.name === name ? { ...r, active: false } : r,
    ),
  // no manifestSync — disable doesn't change the manifest
});
if (!applyResult.ok) return applyResult;
```

### Example 5: Null-abort when condition not met
```typescript
const applyResult = await applySkillStateChange({
  scope,
  projectRoot,
  mutate: (current) => {
    if (!current.some((r) => r.name === name)) return null; // nothing to do
    return current.filter((r) => r.name !== name);
  },
});
// applyResult.ok is still true when null was returned — value is the unchanged array
```

## When to Use

- Any write to the skills slice of `state.json` that needs to be atomic and hook-aware
- Add, remove, update, or disable skill records — all go through `applySkillStateChange`
- When you need manifest sync side effects after a state mutation

## When NOT to Use

- Read-only access to state — use `loadSkillState()` or `loadState()` directly
- Full-state mutations (plugins, mcps) — use `saveState()` after loading with `loadState()`
- Test helpers that need direct control — use `saveSkillState()` directly

## Common Violations

- Calling `loadSkillState` + `saveSkillState` directly without diffing — misses manifest sync
- Throwing inside `mutate()` — should return `null` to abort cleanly
- Forgetting `if (!applyResult.ok) return applyResult` after the call
- Setting `manifestSync` without `projectRoot` — hooks silently never fire
