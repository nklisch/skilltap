# Zod 4 Type System — Gotchas and Patterns

Read this when writing TypeScript that uses Zod schemas as types, passes schemas to functions, or does anything beyond basic `.parse()` calls.

## Table of Contents

1. [ZodType generics changed](#1-zodtype-generics-changed)
2. [ZodTypeAny is gone](#2-zodtypeany-is-gone)
3. [Accepting schemas in function parameters](#3-accepting-schemas-in-function-parameters)
4. [$ZodType vs ZodType](#4-zodtype-vs-zodtype)
5. [.refine() no longer narrows types](#5-refine-no-longer-narrows-types)
6. [.default() matches output type, not input](#6-default-matches-output-type-not-input)
7. [Optional + default behavior changed](#7-optional--default-behavior-changed)
8. [z.unknown()/z.any() are required in objects](#8-zunknownzany-are-required-in-objects)
9. [.nonempty() lost its tuple inference](#9-nonempty-lost-its-tuple-inference)
10. [Record with enum keys are required, not partial](#10-record-with-enum-keys-are-required-not-partial)
11. [.transform() returns ZodPipe now](#11-transform-returns-zodpipe-now)
12. [Branded types — no ZodBranded class](#12-branded-types--no-zodbranded-class)
13. [Metadata is instance-specific](#13-metadata-is-instance-specific)
14. [z.infer vs z.input vs z.output](#14-zinfer-vs-zinput-vs-zoutput)

---

## 1. ZodType generics changed

```typescript
// ZOD 3:
class ZodType<Output, Def extends ZodTypeDef, Input = Output>

// ZOD 4:
class ZodType<Output = unknown, Input = unknown>
```

The `Def` generic is gone entirely. If you had code that referenced all three generics, it won't compile.

```typescript
// ZOD 3 — worked:
type MySchema = z.ZodType<string, z.ZodStringDef, string>

// ZOD 4 — fix:
type MySchema = z.ZodType<string, string>
// Or just:
type MySchema = z.ZodType<string>
```

---

## 2. ZodTypeAny is gone

In Zod 3, `z.ZodTypeAny` was the go-to for "any schema". It's eliminated in Zod 4.

```typescript
// ZOD 3:
function process(schema: z.ZodTypeAny) { ... }

// ZOD 4:
function process(schema: z.ZodType) { ... }
```

---

## 3. Accepting schemas in function parameters

This is the biggest practical gotcha for library/utility code. There are two patterns, and one of them is wrong.

**WRONG — constraining the output type loses subclass info:**
```typescript
// DON'T DO THIS
function validate<T>(schema: z.ZodType<T>, data: unknown): T {
  return schema.parse(data)
}

const s = z.string().min(3)
validate(s, "hi")
// Works but `s` is widened — you can't chain `.min()` etc. on the return
```

**RIGHT — extend the base type to preserve specificity:**
```typescript
// DO THIS
function validate<T extends z.ZodType>(schema: T, data: unknown): z.infer<T> {
  return schema.parse(data)
}

const s = z.string().min(3)
validate(s, "hi")
// Full type info preserved
```

**Constraining to specific schema subclasses:**
```typescript
// Only accept object schemas:
function processObject<T extends z.ZodObject<any>>(schema: T): z.infer<T> { ... }

// Only accept schemas that output strings:
function processString<T extends z.ZodType<string>>(schema: T) { ... }
```

---

## 4. $ZodType vs ZodType

Zod 4 has a split between `zod/v4` and `zod/v4/core`:

- **`zod/v4`** exports `ZodType` — has `.parse()`, `.safeParse()`, etc.
- **`zod/v4/core`** exports `$ZodType` — the base class without parse methods

For library code that accepts schemas from users (who might use `zod/v4` or `zod/v4-mini`), constrain with `$ZodType` from core:

```typescript
import * as z4 from "zod/v4/core"

function validate<T extends z4.$ZodType>(schema: T, data: unknown) {
  return z4.parse(schema, data)  // Use top-level parse, not schema.parse()
}
```

For application code (like skilltap), just use `z.ZodType` from `"zod/v4"` — you control the import.

---

## 5. .refine() no longer narrows types

In Zod 3, a type predicate in `.refine()` would narrow the output type. Zod 4 ignores type predicates entirely.

```typescript
// ZOD 3 — output type narrowed to string:
const narrowed = z.unknown().refine((val): val is string => typeof val === "string")
type Result = z.infer<typeof narrowed> // string

// ZOD 4 — output type stays unknown:
const notNarrowed = z.unknown().refine((val): val is string => typeof val === "string")
type Result = z.infer<typeof notNarrowed> // unknown
```

**Fix:** Use `.pipe()` or `.transform()` with a type assertion if you need narrowing:

```typescript
// Option 1: pipe through the target type
const narrowed = z.unknown().pipe(z.string())

// Option 2: transform with assertion
const narrowed = z.unknown().transform((val) => val as string)
```

---

## 6. .default() matches output type, not input

```typescript
// ZOD 3: default matched the INPUT type
z.string().transform(val => val.length).default("hello")  // default is string (input)

// ZOD 4: default matches the OUTPUT type
z.string().transform(val => val.length).default(0)         // default is number (output)
z.string().transform(val => val.length).default("hello")   // TYPE ERROR
```

If you need pre-parse defaults (the old behavior), use `.prefault()`:

```typescript
// .prefault() applies BEFORE parsing — matches input type
z.string().prefault("hello").transform(val => val.length)
```

---

## 7. Optional + default behavior changed

This is subtle and can cause data bugs if you don't know about it.

```typescript
const schema = z.object({
  a: z.string().default("tuna").optional(),
})

// ZOD 3: parsing {} gives {}               (optional wins, key absent)
// ZOD 4: parsing {} gives { a: "tuna" }    (default wins, key present)
```

In Zod 4, `.default()` short-circuits when input is `undefined`. Since optional fields have `undefined` as a valid input, the default kicks in. The order of `.default()` and `.optional()` no longer matters the way it used to.

---

## 8. z.unknown()/z.any() are required in objects

```typescript
const schema = z.object({
  a: z.any(),
  b: z.unknown(),
})

// ZOD 3 inferred: { a?: any; b?: unknown }      (key-optional)
// ZOD 4 inferred: { a: any; b: unknown }         (key-required)
```

If you want the Zod 3 behavior (key optional), use `.optional()` explicitly:

```typescript
z.object({
  a: z.any().optional(),
  b: z.unknown().optional(),
})
```

---

## 9. .nonempty() lost its tuple inference

```typescript
// ZOD 3:
z.array(z.string()).nonempty()
// Inferred: [string, ...string[]]   (guaranteed at least one element at type level)

// ZOD 4:
z.array(z.string()).nonempty()
// Inferred: string[]                 (just an alias for .min(1), no tuple magic)
```

If you need the `[T, ...T[]]` type for compile-time safety, use a tuple with rest:

```typescript
z.tuple([z.string()], z.string())
// Inferred: [string, ...string[]]
```

---

## 10. Record with enum keys are required, not partial

```typescript
const schema = z.record(z.enum(["a", "b", "c"]), z.number())

// ZOD 3 inferred: { a?: number; b?: number; c?: number }   (partial)
// ZOD 4 inferred: { a: number; b: number; c: number }       (required)
```

For optional enum keys, use `z.partialRecord()`:

```typescript
z.partialRecord(z.enum(["a", "b", "c"]), z.number())
// Inferred: { a?: number; b?: number; c?: number }
```

---

## 11. .transform() returns ZodPipe now

```typescript
const schema = z.string().transform(val => val.length)

// ZOD 3: typeof schema → ZodEffects<ZodString, number, string>
// ZOD 4: typeof schema → ZodPipe<ZodString, ZodTransform>
```

If you had code checking `instanceof ZodEffects`, it'll break. `ZodEffects` is gone — refinements now live directly in schemas, and transforms use `ZodPipe` + `ZodTransform`.

---

## 12. Branded types — no ZodBranded class

```typescript
const UserId = z.string().brand<"UserId">()

// ZOD 3: typeof UserId → ZodBranded<ZodString, "UserId">
// ZOD 4: typeof UserId → ZodString (with branded output type)
```

The `.brand()` method still works for type-level branding, but there's no `ZodBranded` wrapper class. Branding is handled via direct type modification on the output type.

---

## 13. Metadata is instance-specific

`.meta()` attaches metadata to a specific schema instance. Since Zod methods are immutable (they return new instances), transformations create new instances **without** the metadata:

```typescript
const base = z.string().meta({ description: "A name" })
const optional = base.optional()

base.meta()      // { description: "A name" }
optional.meta()  // undefined — new instance, metadata lost
```

You need to re-apply `.meta()` after chaining if the metadata matters:

```typescript
const optional = base.optional().meta({ description: "An optional name" })
```

---

## 14. z.infer vs z.input vs z.output

```typescript
const schema = z.string().transform(val => val.length)

type In  = z.input<typeof schema>   // string   (what you pass in)
type Out = z.output<typeof schema>  // number   (what you get back)
type Inf = z.infer<typeof schema>   // number   (alias for z.output)
```

Use `z.input<>` when typing function parameters that accept raw data before validation. Use `z.infer<>` (or `z.output<>`) for the validated result.

```typescript
// Pattern for typed handler functions:
function handleUser(raw: z.input<typeof UserSchema>) {
  const user: z.infer<typeof UserSchema> = UserSchema.parse(raw)
  // ...
}
```

---

## Internal type detection (for library code)

Distinguish Zod 3 from Zod 4 schemas at runtime:

```typescript
if ("_zod" in schema) {
  // Zod 4 — internal state at schema._zod.def
} else {
  // Zod 3 — internal state at schema._def
}
```

---

## Quick reference: Zod 4 type substitutions

| Zod 3 type | Zod 4 replacement |
|------------|-------------------|
| `z.ZodTypeAny` | `z.ZodType` |
| `z.ZodType<O, D, I>` | `z.ZodType<O, I>` |
| `z.ZodTypeDef` | Removed |
| `z.ZodEffects<S, O, I>` | `z.ZodPipe<S, z.ZodTransform>` |
| `z.ZodBranded<S, B>` | Just `S` (branding is on output type) |
| `z.ZodPreprocess` | `z.ZodPipe` |
| `schema._def` | `schema._zod.def` |
| `z.infer<T>` | `z.infer<T>` (unchanged, alias for `z.output`) |
