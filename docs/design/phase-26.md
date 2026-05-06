# Phase 26 — v2.0 Schema Foundation

## Goal

All v2.0 Zod schemas exist, parse, validate, and have unit tests. v1.0 schemas remain unchanged. No behavior is wired in — Phase 26 is purely additive data-layer work.

## Approach

The schemas are already specified in [SPEC.md — v2.0](../SPEC.md#v20--tooling-surface-redesign) and [ARCH.md — v2.0 Schemas](../ARCH.md#v20-schemas-zod-4). This phase translates those specs into Zod 4 code with tests.

**Deviation from ROADMAP.md 26.3:** The roadmap says "Keep v1.0 schemas in `core/src/schemas/v1/`." I am NOT moving v1.0 schemas in this phase. Reason: v1.0 schemas are heavily depended on across `install.ts`, `update.ts`, `policy.ts`, `lifecycle.ts`, etc. Moving them would require renaming imports across dozens of files, ballooning Phase 26 well beyond its stated scope. Instead, Phase 27 (migration) will add explicit v1.0 schema *copies* under `schemas/v1/` ONLY for the keys the migration command needs to read (config keys, installed.json shape, plugins.json shape). Existing in-place schemas continue to evolve through Phases 31+.

## Implementation Units

### Unit 1 — `core/src/manifest/schemas.ts`

```typescript
import { z } from "zod/v4";

export const TargetsSchema = z.object({
  also: z.array(z.string()).default([]),
  scope: z.enum(["", "global", "project"]).default(""),
}).prefault({});

// A manifest entry value is either a range string ("^1.0", "*", "v1.2.3")
// or an inline table with optional ref + per-component overrides.
export const ManifestEntryDetailSchema = z.object({
  ref: z.string().optional(),
  components: z.record(z.string(), z.boolean()).optional(),
});

export const ManifestEntrySchema = z.union([
  z.string(),
  ManifestEntryDetailSchema,
]);

export const ProjectManifestSchema = z.object({
  targets: TargetsSchema,
  skills: z.record(z.string(), ManifestEntrySchema).default({}),
  plugins: z.record(z.string(), ManifestEntrySchema).default({}),
  taps: z.record(z.string(), z.string()).default({}),
});

export const LockEntrySchema = z.object({
  source: z.string(),
  ref: z.string(),
  sha: z.string().optional(),
  range: z.string(),
});

export const LockfileSchema = z.object({
  version: z.literal(1),
  skill: z.array(LockEntrySchema).default([]),
  plugin: z.array(LockEntrySchema).default([]),
});

export type Targets = z.infer<typeof TargetsSchema>;
export type ManifestEntryDetail = z.infer<typeof ManifestEntryDetailSchema>;
export type ManifestEntry = z.infer<typeof ManifestEntrySchema>;
export type ProjectManifest = z.infer<typeof ProjectManifestSchema>;
export type LockEntry = z.infer<typeof LockEntrySchema>;
export type Lockfile = z.infer<typeof LockfileSchema>;
```

### Unit 2 — `core/src/manifest/range.ts`

Pure-function range parser/matcher. Supports:

- `"*"` — matches any ref/version
- exact match — `"v1.2.3"`, `"main"`, `"abc123"`
- caret — `"^1.0"`, `"^1.2.3"` — semver-compatible (same major)
- tilde — `"~1.2"`, `"~1.2.3"` — same minor
- range tag `"latest"` — alias for `"*"`

API:
```typescript
parseRange(input: string): ParsedRange
matchesRange(range: ParsedRange, candidate: string): boolean
findBestMatch(range: ParsedRange, candidates: string[]): string | null
```

For non-semver candidates (`main`, sha-likes), only exact match works. The matcher gracefully reports "no match" rather than throwing.

### Unit 3 — `core/src/plugin-v2/schema.ts`

```typescript
import { z } from "zod/v4";

export const PluginV2SkillSchema = z.object({
  name: z.string(),
  path: z.string(),
  description: z.string().default(""),
});

export const PluginV2StdioServerSchema = z.object({
  type: z.literal("stdio").default("stdio"),
  name: z.string(),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const PluginV2HttpServerSchema = z.object({
  type: z.literal("http"),
  name: z.string(),
  url: z.string(),
  headers: z.record(z.string(), z.string()).default({}),
});

export const PluginV2ServerSchema = z.union([
  PluginV2StdioServerSchema,
  PluginV2HttpServerSchema,
]);

export const PluginV2AgentSchema = z.object({
  name: z.string(),
  path: z.string(),
});

export const PluginManifestV2Schema = z.object({
  name: z.string().regex(/^[a-z0-9]+(-[a-z0-9]+)*$/),
  version: z.string(),
  description: z.string().default(""),
  publish: z.boolean().default(false),
  skills: z.array(PluginV2SkillSchema).default([]),
  servers: z.array(PluginV2ServerSchema).default([]),
  agents: z.array(PluginV2AgentSchema).default([]),
});
```

### Unit 4 — `core/src/schemas/config-v2.ts`

Mirror of the SPEC.md v2.0 config block, simplified.

```typescript
export const SECURITY_SCAN_V2 = ["semantic", "static", "none"] as const;
export const SECURITY_ON_WARN_V2 = ["prompt", "fail", "install"] as const;
export const SCOPE_V2 = ["", "global", "project"] as const;

export const SecurityConfigV2Schema = z.object({
  scan: z.enum(SECURITY_SCAN_V2).default("static"),
  on_warn: z.enum(SECURITY_ON_WARN_V2).default("install"),
  trust: z.array(z.string()).default([]),
}).prefault({});

export const AgentConfigSchema = z.object({
  default: z.boolean().default(false),
  block: z.boolean().default(false),
}).prefault({});

export const ConfigV2DefaultsSchema = z.object({
  also: z.array(z.string()).default([]),
  scope: z.enum(SCOPE_V2).default(""),
}).prefault({});

// Reuse v1.0 UpdatesConfigSchema, TelemetryConfigSchema, taps shape.
export const ConfigV2Schema = z.object({
  defaults: ConfigV2DefaultsSchema,
  agent: AgentConfigSchema,
  security: SecurityConfigV2Schema,
  taps: z.array(z.object({ name: z.string(), url: z.string() })).default([]),
  updates: UpdatesConfigSchema.prefault({}),  // imported from v1.0 schemas/config.ts
  telemetry: TelemetryConfigSchema.prefault({}),
  builtin_tap: z.boolean().default(true),
  verbose: z.boolean().default(true),
  default_git_host: z.string().default("https://github.com"),
});
```

### Unit 5 — `core/src/state/schema.ts`

```typescript
import { z } from "zod/v4";
import { InstalledSkillSchema } from "../schemas/installed";
import { PluginRecordSchema } from "../schemas/plugins";

export const StoredMcpStandaloneSchema = z.object({
  name: z.string(),
  source: z.string(),
  // Reuse stdio/http server union from existing plugin schemas:
  config: z.union([
    z.object({
      type: z.literal("stdio").default("stdio"),
      command: z.string(),
      args: z.array(z.string()).default([]),
      env: z.record(z.string(), z.string()).default({}),
    }),
    z.object({
      type: z.literal("http"),
      url: z.string(),
      headers: z.record(z.string(), z.string()).default({}),
    }),
  ]),
  targets: z.array(z.string()).default([]),
  installedAt: z.iso.datetime(),
});

export const StateSchema = z.object({
  version: z.literal(2),
  skills: z.array(InstalledSkillSchema).default([]),
  plugins: z.array(PluginRecordSchema).default([]),
  mcpServers: z.array(StoredMcpStandaloneSchema).default([]),
});

export type StoredMcpStandalone = z.infer<typeof StoredMcpStandaloneSchema>;
export type State = z.infer<typeof StateSchema>;
```

### Unit 6–10 — Test files

Each schema file gets a `*.test.ts` next to it. Tests cover:

- **Round-trip**: parse a known-good fixture, expect it equals expected shape.
- **Defaults**: parse `{}` (or empty) and verify nested defaults applied.
- **Invalid input**: parse bad shapes, expect `safeParse` to fail with a clear path.
- **Range parser**: parse + match across `*`, exact, caret, tilde, non-semver. Best-match selection across a candidate list.

## Verification

```bash
bun test packages/core/src/manifest/
bun test packages/core/src/plugin-v2/
bun test packages/core/src/schemas/config-v2.test.ts
bun test packages/core/src/state/
```

All tests pass. No existing tests should fail (schemas are purely additive).

## Out of Scope

- Loaders (`load.ts`, `save.ts`) — deferred to Phase 27/28.
- Migration logic — Phase 27.
- Wiring schemas into install/update/sync flows — Phase 28+.
- Removing v1.0 schemas — never (they stay; just not used in v2.0 paths after Phase 31).
