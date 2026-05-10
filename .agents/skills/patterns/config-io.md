# Pattern: Config/State Load-Save Algorithm

Config and state files follow a consistent algorithm: ensure directories exist → check file existence → read or return default → validate with Zod → return Result.

## Rationale

Config files may not exist yet (first run), may be corrupt, or may have a stale format. The algorithm handles all three gracefully and never leaves the system in a half-initialized state.

## Examples

### Example 1: Load config (TOML)
**File**: `packages/core/src/config.ts:87`
```typescript
export async function loadConfig(): Promise<Result<Config, UserError>> {
  const dirsResult = await ensureDirs()
  if (!dirsResult.ok) return dirsResult          // 1. ensure dirs exist

  const configPath = join(configDir(), "config.toml")
  const exists = await Bun.file(configPath).exists()
  if (!exists) {                                  // 2. if missing, write template + return default
    await Bun.write(configPath, CONFIG_TEMPLATE)
    return ok(structuredClone(DEFAULT_CONFIG))
  }

  const text = await Bun.file(configPath).text() // 3. read file
  let raw: unknown
  try {
    raw = parse(text)                             // 4. parse format (TOML/JSON)
  } catch (e) {
    return err(new UserError(`Failed to parse config.toml: ${e}`))
  }

  const result = ConfigSchema.safeParse(raw)      // 5. validate with Zod
  if (!result.success) {
    return err(new UserError(`Invalid config:\n${z.prettifyError(result.error)}`))
  }
  return ok(result.data)                          // 6. return ok
}
```

### Example 2: Load state.json (JSON via loadJsonState helper)
**File**: `packages/core/src/state/load.ts:13`
```typescript
export async function loadState(
  projectRoot?: string,
): Promise<Result<State, UserError>> {
  return loadJsonState(          // generic JSON helper handles exists check + Zod validation
    getStatePath(projectRoot),
    StateSchema,
    "state.json",
    DEFAULT_STATE,               // returned as-is when file is absent
  );
}
```
State is accessed through `loadSkillState()` (skills slice) or `loadState()` (full state including plugins and MCP servers). The old `installed.json` and `loadInstalled()` no longer exist — `state.json` is the sole store.

### Example 3: Save config
**File**: `packages/core/src/config.ts:128`
```typescript
export async function saveConfig(config: Config): Promise<Result<void, UserError>> {
  const dirsResult = await ensureDirs()
  if (!dirsResult.ok) return dirsResult

  const configPath = join(configDir(), "config.toml")
  await Bun.write(configPath, stringify(config))   // Bun.write, no try/catch — let it throw
  return ok(undefined)
}
```

## When to Use

- Any persistent file that the system owns and may need to create on first run
- Config, state, or lock files that can be missing, corrupt, or outdated

## When NOT to Use

- Temp files — use `makeTmpDir()` + direct `Bun.write`
- Read-only files you don't own (tap repositories) — validate but don't auto-create defaults

## Common Violations

- Skipping `ensureDirs()` — causes write failures on first run
- Using `parse()` instead of `safeParse()` — throws uncaught exception
- Returning the default without writing the template — leaves the file missing for next run
- Calling `JSON.parse` without try/catch — Zod won't catch JSON syntax errors
