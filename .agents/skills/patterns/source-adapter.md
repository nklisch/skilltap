# Pattern: SourceAdapter Strategy

Source types (git URL, GitHub shorthand, local path) are implemented as plain object literals conforming to a `SourceAdapter` interface. A resolver iterates registered adapters in priority order.

## Rationale

New source types can be added by implementing three fields on a plain object — no inheritance, no registration ceremony. `canHandle()` separates detection from resolution, keeping each adapter focused. Order in the resolver array controls priority.

## Examples

### Example 1: Interface definition
**File**: `packages/core/src/adapters/types.ts:4`
```typescript
export interface SourceAdapter {
  readonly name: string
  canHandle(source: string): boolean
  resolve(source: string): Promise<Result<ResolvedSource, UserError>>
}
```

### Example 2: Git adapter (simple URL prefix matching)
**File**: `packages/core/src/adapters/git.ts:6`
```typescript
const URL_PREFIXES = ["https://", "http://", "git@", "ssh://"]

export const gitAdapter: SourceAdapter = {
  name: "git",
  canHandle(source: string): boolean {
    return URL_PREFIXES.some(p => source.startsWith(p))
  },
  async resolve(source: string): Promise<Result<ResolvedSource, UserError>> {
    return ok({ url: source, adapter: "git" })
  },
}
```

### Example 3: GitHub adapter (shorthand parsing with error cases)
**File**: `packages/core/src/adapters/github.ts:7`
```typescript
export const githubAdapter: SourceAdapter = {
  name: "github",
  canHandle(source: string): boolean {
    return GITHUB_PATTERN.test(source)
  },
  async resolve(source: string): Promise<Result<ResolvedSource, UserError>> {
    const match = source.match(GITHUB_PATTERN)
    if (!match) return err(new UserError("Invalid GitHub source format"))
    const [, owner, repo, ref] = match
    const url = `https://github.com/${owner}/${repo}.git`
    return ok({ url, ...(ref ? { ref } : {}), adapter: "github" })
  },
}
```

### Example 4: Resolver — priority-ordered adapter loop
**File**: `packages/core/src/adapters/resolve.ts:10`
```typescript
const ADAPTERS: SourceAdapter[] = [gitAdapter, npmAdapter, localAdapter, githubAdapter]

export async function resolveSource(source: string): Promise<Result<ResolvedSource, UserError>> {
  for (const adapter of ADAPTERS) {
    if (adapter.canHandle(source)) {
      return adapter.resolve(source)
    }
  }
  return err(new UserError(`Unrecognized source: ${source}`))
}
```

## When to Use

- Adding a new source type (e.g., GitLab shorthand, Gitea, npm package)
- Any "dispatch on input shape" problem with 3+ variants

## When NOT to Use

- If there will only ever be one implementation — no need for an interface
- For adapters that need shared mutable state — plain objects don't support that cleanly

## Common Violations

- Implementing adapters as classes — object literals are simpler and sufficient
- Putting detection logic inside `resolve()` instead of `canHandle()` — keeps adapter selection separate from resolution
- Hardcoding adapter names in the resolver instead of using the `ADAPTERS` array
