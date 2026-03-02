import { z } from "zod/v4";

export const RegistrySourceSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("git"), url: z.string(), ref: z.string().optional() }),
  z.object({ type: z.literal("github"), repo: z.string(), ref: z.string().optional() }),
  z.object({ type: z.literal("npm"), package: z.string(), version: z.string().optional() }),
  z.object({ type: z.literal("url"), url: z.string() }),
]);

export const RegistryTrustSchema = z.object({
  verified: z.boolean().default(false),
  verifiedBy: z.string().optional(),
});

export const RegistrySkillSchema = z.object({
  name: z.string(),
  description: z.string(),
  version: z.string().optional(),
  author: z.string().optional(),
  tags: z.array(z.string()).default([]),
  source: RegistrySourceSchema,
  trust: RegistryTrustSchema.optional(),
});

export const RegistryListResponseSchema = z.object({
  skills: z.array(RegistrySkillSchema),
  total: z.number().int().optional(),
  cursor: z.string().optional(),
});

export const RegistryDetailResponseSchema = z.object({
  name: z.string(),
  description: z.string(),
  author: z.string().optional(),
  license: z.string().optional(),
  tags: z.array(z.string()).default([]),
  versions: z
    .array(
      z.object({
        version: z.string(),
        publishedAt: z.string().optional(),
      }),
    )
    .default([]),
  source: RegistrySourceSchema,
  trust: RegistryTrustSchema.optional(),
});

export type RegistrySource = z.infer<typeof RegistrySourceSchema>;
export type RegistrySkill = z.infer<typeof RegistrySkillSchema>;
export type RegistryListResponse = z.infer<typeof RegistryListResponseSchema>;
export type RegistryDetailResponse = z.infer<typeof RegistryDetailResponseSchema>;
