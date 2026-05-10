# Pattern: scopeBase Path Helper

`scopeBase(scope, projectRoot?)` is the single-source resolver for scope-to-base-directory mapping. All scope-conditional directory construction goes through this function instead of inline ternaries.

## Rationale

Several modules need to compute `~/.config/skilltap/` (global) vs `<projectRoot>/` (project) as a base for further path construction. Inlining `scope === "global" ? globalBase() : (projectRoot ?? cwd())` in multiple places creates drift. `scopeBase` centralizes the logic in `paths.ts`; callers that need full skill/plugin paths use the derived helpers (`skillInstallDir`, `skillDisabledDir`, `agentDefPath`) which call `scopeBase` internally.

## Examples

### Example 1: scopeBase definition
**File**: `packages/core/src/paths.ts:8`
```typescript
export function scopeBase(
  scope: "global" | "project",
  projectRoot?: string,
): string {
  return scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
}
```

### Example 2: Internal use — derived path helpers
**File**: `packages/core/src/paths.ts:57`
```typescript
export function skillInstallDir(
  name: string,
  scope: "global" | "project",
  projectRoot?: string,
): string {
  return join(scopeBase(scope, projectRoot), ".agents", "skills", name);
}

export function skillDisabledDir(name: string, scope, projectRoot?): string {
  return join(scopeBase(scope, projectRoot), ".agents", "skills", ".disabled", name);
}
```

### Example 3: Direct use in adopt.ts for base-dir logic
**File**: `packages/core/src/adopt.ts:224`
```typescript
const base = scopeBase(scope, projectRoot);
const skillsDir = join(base, ".agents", "skills");
```

### Example 4: MCP injection uses scopeBase for project root
**File**: `packages/core/src/plugin/mcp-inject.ts:120`
```typescript
const base = scopeBase("project", projectRoot);
const agentsDir = join(base, ".agents");
```

## When to Use

- Any code that computes a base directory that depends on `scope`
- Prefer the derived helpers (`skillInstallDir`, `agentDefPath`, etc.) when building skill/agent paths — they call `scopeBase` internally
- Use `scopeBase` directly only when building a path not covered by an existing helper

## When NOT to Use

- Don't inline `scope === "global" ? globalBase() : projectRoot` — use `scopeBase`
- Don't call `process.cwd()` directly in scope-conditional code — `scopeBase` handles the fallback

## Common Violations

- Inlining the ternary instead of calling `scopeBase` — creates divergence when fallback behavior changes
- Passing `undefined` as `projectRoot` for project-scoped operations — always pass the actual project root
- Building paths on top of `globalBase()` or `projectRoot` directly for scope-conditional logic
