# Pattern: Error Class Hierarchy

Typed error subclasses with an optional `hint` field categorize failures so callers and the CLI layer can react appropriately.

## Rationale

A flat `Error` string is opaque to callers. Typed subclasses (`UserError`, `GitError`, etc.) let the CLI distinguish between "show the user a friendly message" and "show a git stderr dump". The `hint` field carries actionable next-step advice separate from the raw error message.

## Examples

### Example 1: Base class and subclasses
**File**: `packages/core/src/types.ts:13`
```typescript
export class SkilltapError extends Error {
  readonly hint?: string
  constructor(message: string, options?: { hint?: string }) {
    super(message)
    this.name = "SkilltapError"
    this.hint = options?.hint
  }
}

export class UserError extends SkilltapError {
  constructor(message: string, hint?: string) {
    super(message, { hint })
    this.name = "UserError"
  }
}

export class GitError extends SkilltapError {
  constructor(message: string, hint?: string) {
    super(message, { hint })
    this.name = "GitError"
  }
}

export class ScanError extends SkilltapError { /* same shape */ }
export class NetworkError extends SkilltapError { /* same shape */ }
```

### Example 2: Constructing errors with hints
**File**: `packages/core/src/git.ts:68`
```typescript
return err(new GitError(
  `git fetch failed: ${extractStderr(e)}`,
  "Check your network connection and repository URL"
))
```

### Example 3: UserError from validation failure
**File**: `packages/core/src/config.ts:101`
```typescript
return err(new UserError(
  `Failed to parse config: ${z.prettifyError(result.error)}`,
  `Fix the config file at ${configPath}`
))
```

### Example 4: UserError without hint (simple case)
**File**: `packages/core/src/adapters/local.ts:23`
```typescript
return err(new UserError(`Path does not exist: ${resolved}`))
```

## When to Use

- `UserError` — problem caused by user input (bad URL, missing file, invalid config)
- `GitError` — git subprocess failure
- `ScanError` — skill discovery/parsing failure
- `NetworkError` — network/API request failure

## When NOT to Use

- Don't throw these — they're always wrapped in `err()`
- Don't use base `SkilltapError` directly — always use a subclass
- Don't catch-and-rethrow in core — propagate the original error

## Common Violations

- `new Error("message")` instead of typed subclass — loses category info
- Putting actionable steps in `message` instead of `hint`
- `throw new UserError(...)` instead of `return err(new UserError(...))`
