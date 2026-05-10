# Pattern: JSON State I/O

`loadJsonState<T>` and `saveJsonState` are generic helpers in `json-state.ts` that implement the shared read-validate-default / ensure-dirs-write lifecycle for all JSON state files. All state modules delegate to these rather than doing ad-hoc JSON file I/O.

## Rationale

State files (`state.json`, global and project) share the same needs: check existence, parse JSON, validate with Zod, return a default if missing, create directories before writing. Centralizing in generic helpers prevents drift and ensures every state file gets the same error messaging and dir-creation behavior.

## Examples

### Example 1: loadJsonState signature
**File**: `packages/core/src/json-state.ts:7`
```typescript
export async function loadJsonState<T>(
  path: string,
  schema: z.ZodType<T>,
  label: string,
  defaultValue: T,
): Promise<Result<T, UserError>> {
  const f = Bun.file(path);
  if (!(await f.exists())) return ok(defaultValue);
  let raw: unknown;
  try {
    raw = await f.json();
  } catch (e) {
    return err(new UserError(`Invalid JSON in ${label}: ${e}`));
  }
  return parseWithResult(schema, raw, label);
}
```

### Example 2: saveJsonState signature
**File**: `packages/core/src/json-state.ts:24`
```typescript
export async function saveJsonState(
  path: string,
  data: unknown,
  label: string,
  projectRoot: string | undefined,
  ensureGlobalDirs: () => Promise<Result<void, UserError>>,
): Promise<Result<void, UserError>> {
  if (projectRoot) {
    await mkdir(join(projectRoot, ".agents"), { recursive: true });
  } else {
    const dirsResult = await ensureGlobalDirs();
    if (!dirsResult.ok) return dirsResult;
  }
  await Bun.write(path, JSON.stringify(data, null, 2));
  return ok(undefined);
}
```
- Project scope: creates `<projectRoot>/.agents/` (idempotent)
- Global scope: delegates to `ensureDirs()` for `~/.config/skilltap/`

### Example 3: loadState delegates to loadJsonState
**File**: `packages/core/src/state/load.ts:13`
```typescript
export async function loadState(
  projectRoot?: string,
): Promise<Result<State, UserError>> {
  return loadJsonState(
    getStatePath(projectRoot),
    StateSchema,
    "state.json",
    DEFAULT_STATE,
  );
}
```

### Example 4: saveState delegates to saveJsonState
**File**: `packages/core/src/state/save.ts:7`
```typescript
export async function saveState(
  state: State,
  projectRoot?: string,
): Promise<Result<void, UserError>> {
  return saveJsonState(
    getStatePath(projectRoot),
    state,
    "state.json",
    projectRoot,
    ensureDirs,
  );
}
```

## When to Use

- Any new JSON state file the system owns (e.g., a new per-scope state slice)
- Implement a new module like `state/load.ts` + `state/save.ts` that delegates to these helpers
- Never read/write JSON state files with raw `Bun.file().json()` in a state module

## When NOT to Use

- TOML config files — those use `smol-toml`'s `parse`/`stringify` + `loadConfig`/`saveConfig`
- Temp files or files you don't own (tap repositories) — use `Bun.write` directly
- Read-only files (fixture data in tests) — no need for the full load lifecycle

## Common Violations

- Ad-hoc `Bun.file(path).json()` in state modules bypassing the helper — misses error handling and dir creation
- Adding a new state file without the `loadX`/`saveX` wrapper that delegates to `loadJsonState`/`saveJsonState`
- Passing `undefined` as `projectRoot` for a project-scoped save — global dirs will be created instead
