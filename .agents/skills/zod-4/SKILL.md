---
name: zod-4
description: Reference for Zod 4 schema validation. Use this skill whenever defining schemas, validating data, or working with Zod. IMPORTANT — Zod 4 has major API changes from Zod 3. This project uses `import { z } from "zod/v4"`. Key changes include z.record requiring two args, string validators moving to top-level (z.email()), .strict()/.passthrough() replaced by z.strictObject()/z.looseObject(), .merge() deprecated in favor of .extend(), error handling overhauled. Use this skill for any file in packages/core/src/schemas/ or anywhere using Zod.
---

# Zod 4 — Schema Validation Reference

This project uses Zod 4. Many Zod 3 patterns are deprecated or changed. Always use the patterns below.

**Type system gotchas:** Read `references/types.md` whenever writing code that passes schemas to functions, uses generics with Zod types, or relies on type inference from `.transform()`, `.default()`, `.refine()`, or `.optional()`. The type system changed significantly — `ZodTypeAny` is gone, generics simplified, `.refine()` no longer narrows, `.default()` matches output type, and more.

**Install:** `bun add zod@^4`

**Import:** `import { z } from "zod/v4"`

The bare `import { z } from "zod"` still gives Zod 3 for backward compatibility. Always use `"zod/v4"` explicitly.

## Breaking Changes from Zod 3

These are the most important changes. Getting these wrong causes runtime errors or type mismatches.

### z.record() requires two arguments

```typescript
// ZOD 3 (WRONG in Zod 4):
z.record(z.string())

// ZOD 4 (CORRECT):
z.record(z.string(), z.string())   // Record<string, string>
z.record(z.string(), z.number())   // Record<string, number>
z.record(z.string(), z.unknown())  // Record<string, unknown>

// Constrained keys with enum:
z.record(z.enum(["a", "b", "c"]), z.number())
// { a: number; b: number; c: number }
```

### String validators moved to top-level

```typescript
// ZOD 3 (deprecated):
z.string().email()
z.string().uuid()
z.string().url()

// ZOD 4 (preferred):
z.email()
z.uuid()
z.url()
z.emoji()
z.base64()
z.base64url()
z.nanoid()
z.cuid()
z.cuid2()
z.ulid()
z.ipv4()        // was z.string().ip()
z.ipv6()        // was z.string().ip()
z.cidrv4()      // was z.string().cidr()
z.cidrv6()      // was z.string().cidr()
z.iso.date()    // was z.string().date()
z.iso.time()    // was z.string().time()
z.iso.datetime() // was z.string().datetime()
z.iso.duration() // was z.string().duration()

// z.string().email() still works but is deprecated
// The old z.string().ip() and z.string().cidr() are REMOVED — use the specific v4/v6 variants
```

### .strict() / .passthrough() replaced

```typescript
// ZOD 3 (deprecated):
z.object({ name: z.string() }).strict()
z.object({ name: z.string() }).passthrough()
z.object({ name: z.string() }).strip()

// ZOD 4:
z.strictObject({ name: z.string() })    // Rejects unknown keys
z.looseObject({ name: z.string() })     // Passes through unknown keys
z.object({ name: z.string() })          // Strips unknown keys (default, same as before)
```

### .merge() deprecated

```typescript
// ZOD 3 (deprecated):
const Combined = SchemaA.merge(SchemaB)

// ZOD 4:
const Combined = SchemaA.extend(SchemaB.shape)
// Or use spread:
const Combined = z.object({ ...SchemaA.shape, ...SchemaB.shape })
```

### z.preprocess() removed

```typescript
// ZOD 3 (removed):
z.preprocess((val) => String(val), z.string())

// ZOD 4 — use .pipe():
z.unknown().pipe(z.coerce.string())
// Or chain pipes:
z.string().pipe(z.coerce.number()).pipe(z.number().int().positive())
```

### z.nativeEnum() deprecated

```typescript
// ZOD 3 (deprecated):
z.nativeEnum(MyEnum)

// ZOD 4 — z.enum() handles both:
z.enum(["a", "b", "c"])     // String array
z.enum(MyEnum)               // TypeScript enum

// No more `as const` needed for arrays:
z.enum(["a", "b", "c"])     // Infers "a" | "b" | "c" without `as const`
```

### .nonempty() changed

```typescript
// ZOD 3: .nonempty() inferred [T, ...T[]] tuple type
// ZOD 4: .nonempty() is just .min(1), infers T[]
z.array(z.string()).nonempty()  // string[] (not [string, ...string[]])

// For tuple "at least one" pattern:
z.tuple([z.string()], z.string())  // [string, ...string[]]
```

### Error handling overhauled

```typescript
// ZOD 3 (deprecated):
error.format()
error.flatten()

// ZOD 4:
z.treeifyError(error)    // Structured tree: { _errors: [...], field: { _errors: [...] } }
z.prettifyError(error)   // Human-readable formatted string

// ZOD 3 (deprecated):
z.setErrorMap(map)

// ZOD 4:
z.config({
  customError: (issue) => {
    if (issue.code === "invalid_type") {
      return { message: `Expected ${issue.expected}, got ${issue.received}` }
    }
  },
})
```

### .default() behavior changed

```typescript
// ZOD 4: .default() short-circuits on undefined, default must match OUTPUT type
z.string().transform(v => v.length).default(0)   // default is number (output type)

// ZOD 4: Use .prefault() for pre-parse defaults (old Zod 3 behavior)
z.string().prefault("hello").transform(v => v.length)
```

### .refine() no longer narrows types

```typescript
// ZOD 3: type predicates in .refine() narrowed the output type
// ZOD 4: .refine() ignores type predicates — no type narrowing
```

### .deepPartial() removed

```typescript
// ZOD 3 (removed):
MySchema.deepPartial()

// ZOD 4: Use .partial() and apply it manually to nested schemas
```

### Internal changes

```typescript
// Generic signature changed:
// ZOD 3: ZodType<Output, Def extends ZodTypeDef, Input>
// ZOD 4: ZodType<Output, Input>

// ZodTypeAny eliminated — use ZodType directly
// ._def moved to ._zod.def
// ZodEffects → dropped; refinements live in schemas
// ZodPreprocess → ZodPipe
```

## New Features in Zod 4

### Unified error parameter

```typescript
// Simple string message:
z.string({ error: "Must be a string" })

// Dynamic message function:
z.string({ error: (issue) => `Expected string, got ${typeof issue.input}` })

// Per-check messages:
z.string().min(3, { error: "Too short" })
```

### Metadata system

```typescript
const UserSchema = z.object({
  name: z.string().meta({ description: "Full name", example: "Jane Doe" }),
  email: z.email().meta({ description: "Primary email" }),
}).meta({ title: "User" })

UserSchema.meta()  // { title: "User" }
```

### Built-in JSON Schema conversion

```typescript
import { toJSONSchema } from "zod/v4/json-schema"

const jsonSchema = toJSONSchema(UserSchema)
// Produces standard JSON Schema, uses .meta() for descriptions
```

### z.literal() accepts arrays

```typescript
// ZOD 3:
z.union([z.literal("active"), z.literal("inactive"), z.literal("pending")])

// ZOD 4:
z.literal(["active", "inactive", "pending"])
// Inferred: "active" | "inactive" | "pending"
```

### z.templateLiteral()

```typescript
z.templateLiteral([z.number(), z.literal("px")])
// Matches "10px", "3.5px" — type: `${number}px`
```

### z.file()

```typescript
z.file().type("image/png").maxSize(5 * 1024 * 1024)
```

### z.stringbool()

```typescript
z.stringbool()
// "true"/"1"/"yes"/"on" → true
// "false"/"0"/"no"/"off" → false
```

### Automatic discriminated union detection

```typescript
// ZOD 3: required z.discriminatedUnion("type", [...])
// ZOD 4: z.union() auto-detects discriminator fields
z.union([
  z.object({ type: z.literal("circle"), radius: z.number() }),
  z.object({ type: z.literal("square"), side: z.number() }),
])
// Internally optimized as discriminated union
```

### z.input<> and z.output<>

```typescript
type Input = z.input<typeof Schema>    // Type before transforms
type Output = z.output<typeof Schema>  // Type after transforms (same as z.infer)
```

### zod/v4-mini

Smaller build (~2KB vs ~13KB) for bundle-sensitive contexts. Same API but no built-in error messages — you must provide your own:

```typescript
import { z } from "zod/v4-mini"
z.string({ error: "Expected string" }).min(1, { error: "Required" })
```

## Core API (unchanged from Zod 3)

These work the same as before:

```typescript
// Primitives
z.string()
z.number()
z.boolean()
z.bigint()
z.date()
z.undefined()
z.null()
z.void()
z.any()
z.unknown()
z.never()

// Objects
z.object({ name: z.string(), age: z.number() })
  .partial()                    // All fields optional
  .partial({ name: true })     // Only name optional
  .required()                  // All fields required
  .pick({ name: true })       // Only name
  .omit({ age: true })        // Everything except age
  .extend({ role: z.string() }) // Add fields
  .keyof()                     // z.enum(["name", "age"])

// Arrays and tuples
z.array(z.string()).min(1).max(10)
z.tuple([z.string(), z.number()])

// Unions and intersections
z.union([z.string(), z.number()])
z.intersection(SchemaA, SchemaB)

// Enums
z.enum(["admin", "user", "moderator"])

// Optional and nullable
z.string().optional()          // string | undefined
z.string().nullable()          // string | null
z.string().nullish()           // string | null | undefined

// Transforms
z.string().transform(val => val.length)

// Refinements
z.string().refine(val => val.length > 0, { message: "Required" })
z.object({ a: z.string(), b: z.string() })
  .superRefine((data, ctx) => {
    if (data.a !== data.b) {
      ctx.addIssue({ code: "custom", message: "Must match", path: ["b"] })
    }
  })

// Coercion
z.coerce.string()   // anything → String(x)
z.coerce.number()   // anything → Number(x)
z.coerce.boolean()  // anything → Boolean(x)
z.coerce.date()     // anything → new Date(x)

// Lazy (recursive schemas)
const CategorySchema: z.ZodType<Category> = z.object({
  name: z.string(),
  children: z.lazy(() => z.array(CategorySchema)),
})

// Parsing
schema.parse(data)              // Returns data or throws ZodError
schema.safeParse(data)          // Returns { success, data } or { success, error }
await schema.parseAsync(data)   // Async version
await schema.safeParseAsync(data)
```

## Pattern: skilltap Schema Definitions

This is how schemas are defined in `packages/core/src/schemas/`:

```typescript
import { z } from "zod/v4"

export const SkillFrontmatterSchema = z.object({
  name: z.string().min(1).max(64).regex(/^[a-z0-9]+(-[a-z0-9]+)*$/),
  description: z.string().min(1).max(1024),
  license: z.string().optional(),
  compatibility: z.string().max(500).optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
})

export type SkillFrontmatter = z.infer<typeof SkillFrontmatterSchema>

// Validate at data boundaries:
const result = SkillFrontmatterSchema.safeParse(parsed)
if (!result.success) {
  console.error(z.prettifyError(result.error))
}
```
