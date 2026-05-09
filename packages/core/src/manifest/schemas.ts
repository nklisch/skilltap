import { z } from "zod/v4";

// Default agent symlink targets and default scope for installs originating
// from this manifest. Project manifests use these in lieu of [defaults] in
// the global config.
export const TargetsSchema = z
  .object({
    also: z.array(z.string()).default([]),
    scope: z.enum(["", "global", "project"]).default(""),
  })
  .prefault({});

// Inline-table form of a manifest entry — allows pinning a ref and
// disabling/enabling individual plugin components from the consumer side.
export const ManifestEntryDetailSchema = z.object({
  ref: z.string().optional(),
  components: z.record(z.string(), z.boolean()).optional(),
});

// A manifest entry value is either a range string ("^1.0", "*", "v1.2.3")
// or an inline table.
export const ManifestEntrySchema = z.union([
  z.string(),
  ManifestEntryDetailSchema,
]);

// Standalone MCP entry — promoted to first-class manifest+lockfile in v2.2.
// Inline-table (single-line) form keyed by the user-chosen install name. The
// `ref` field is the git ref (branch/tag/sha) — exact pin semantics; there is
// no separate `range` field.
export const ManifestMcpEntrySchema = z.object({
  name: z.string().min(1),
  source: z.string().min(1),
  ref: z.string().default("main"),
  also: z.array(z.string()).default([]),
});

export const ProjectManifestSchema = z.object({
  targets: TargetsSchema,
  skills: z.record(z.string(), ManifestEntrySchema).default({}),
  plugins: z.record(z.string(), ManifestEntrySchema).default({}),
  mcps: z.array(ManifestMcpEntrySchema).default([]),
  taps: z.record(z.string(), z.string()).default({}),
});

export const LockEntrySchema = z.object({
  source: z.string(),
  ref: z.string(),
  sha: z.string().optional(),
  range: z.string(),
});

export const LockfileMcpEntrySchema = z.object({
  name: z.string().min(1),
  source: z.string().min(1),
  ref: z.string().min(1),
  sha: z.string().min(1),
  also: z.array(z.string()).default([]),
});

export const LockfileSchema = z.object({
  version: z.literal(1),
  skill: z.array(LockEntrySchema).default([]),
  plugin: z.array(LockEntrySchema).default([]),
  mcps: z.array(LockfileMcpEntrySchema).default([]),
});

export type Targets = z.infer<typeof TargetsSchema>;
export type ManifestEntryDetail = z.infer<typeof ManifestEntryDetailSchema>;
export type ManifestEntry = z.infer<typeof ManifestEntrySchema>;
export type ManifestMcpEntry = z.infer<typeof ManifestMcpEntrySchema>;
export type ProjectManifest = z.infer<typeof ProjectManifestSchema>;
export type LockEntry = z.infer<typeof LockEntrySchema>;
export type LockfileMcpEntry = z.infer<typeof LockfileMcpEntrySchema>;
export type Lockfile = z.infer<typeof LockfileSchema>;
