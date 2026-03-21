# Pattern: Zod Schema as Data Boundary

Every external data source (config files, JSON state, SKILL.md frontmatter, API responses) is validated with a Zod schema at the point of ingestion. The schema is also the source of truth for the TypeScript type — no duplicate interface definitions.

## Rationale

External data is untrustworthy. Validating at boundaries prevents malformed data from propagating into business logic. Deriving types from schemas eliminates type/schema drift — if you change the schema, types update automatically.

## Examples

### Example 1: Schema definition + type inference
**File**: `packages/core/src/schemas/config.ts:3`
```typescript
import { z } from "zod/v4"

export const ConfigSchema = z.object({
  taps: z.record(z.string(), TapConfigSchema).prefault({}),
  defaults: DefaultsSchema.prefault({}),
  security: SecurityConfigSchema.prefault({}),
})

export type Config = z.infer<typeof ConfigSchema>
```

### Example 2: safeParse + prettifyError at a data boundary
**File**: `packages/core/src/config.ts:120`
```typescript
const result = ConfigSchema.safeParse(raw)
if (!result.success) {
  return err(new UserError(
    `Invalid config:\n${z.prettifyError(result.error)}`,
    `Fix ${configPath}`
  ))
}
return ok(result.data)
```

### Example 3: Schema used at installed.json boundary
**File**: `packages/core/src/config.ts:165`
```typescript
const result = InstalledJsonSchema.safeParse(raw)
if (!result.success) {
  return err(new UserError(
    `Corrupt installed.json:\n${z.prettifyError(result.error)}`,
  ))
}
return ok(result.data)
```

### Example 4: Frontmatter schema in scanner (permissive — collects warnings)
**File**: `packages/core/src/scanner.ts:51`
```typescript
const parsed = SkillFrontmatterSchema.safeParse(data)
if (!parsed.success) {
  warnings.push(...parsed.error.issues.map(i => i.message))
  // skill marked invalid but processing continues
}
```

### Example 5: Nested object defaults with prefault
**File**: `packages/core/src/schemas/config.ts:25`
```typescript
// Zod 4: .default({}) short-circuits nested defaults; use .prefault({}) instead
export const ConfigSchema = z.object({
  defaults: DefaultsSchema.prefault({}),
  security: SecurityConfigSchema.prefault({}),
})
```

## When to Use

- Every file/network/process boundary where data enters the system
- Config load, installed.json load, tap.json load, marketplace.json load, SKILL.md frontmatter parse

## When NOT to Use

- Internal data passed between core functions (already validated)
- Test fixture data you control — validate behavior, not the fixture itself

## Common Violations

- `as Config` type assertion instead of `safeParse` — silently skips validation
- `parse()` instead of `safeParse()` — throws uncaught exception from core
- `import { z } from "zod"` — must be `from "zod/v4"`
- `.default({})` on nested objects — use `.prefault({})` in Zod 4
- Defining a separate `interface Config` alongside `ConfigSchema` — schemas are the source of truth
