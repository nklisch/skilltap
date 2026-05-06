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
