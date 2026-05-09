import { z } from "zod/v4";

const NpmDistSchema = z
  .object({
    tarball: z.string(),
    integrity: z.string().optional(),
    shasum: z.string().optional(),
    attestations: z
      .object({ url: z.string(), provenance: z.unknown() })
      .optional(),
  })
  .passthrough();

const NpmVersionEntrySchema = z
  .object({
    version: z.string(),
    dist: NpmDistSchema,
    _npmUser: z.object({ name: z.string().optional() }).optional(),
  })
  .passthrough();

export const NpmPackageMetadataSchema = z
  .object({
    name: z.string().optional(),
    description: z.string().optional(),
    "dist-tags": z.record(z.string(), z.string()).optional(),
    versions: z.record(z.string(), NpmVersionEntrySchema).optional(),
  })
  .passthrough();

export type NpmPackageMetadataRaw = z.infer<typeof NpmPackageMetadataSchema>;
