import { z } from "zod/v4";
import { TrustInfoSchema } from "../trust/types";

export const InstalledSkillSchema = z.object({
  name: z.string(),
  description: z.string().default(""),
  repo: z.string().nullable(),
  ref: z.string().nullable(),
  sha: z.string().nullable().default(null),
  scope: z.enum(["global", "project", "linked"]),
  path: z.string().nullable(),
  tap: z.string().nullable().default(null),
  also: z.array(z.string()).default([]),
  installedAt: z.iso.datetime(),
  updatedAt: z.iso.datetime().default("1970-01-01T00:00:00.000Z"),
  trust: TrustInfoSchema.optional(),
  active: z.boolean().default(true),
});

export const InstalledJsonSchema = z.object({
  version: z.literal(1),
  skills: z.array(InstalledSkillSchema),
});

export type InstalledSkill = z.infer<typeof InstalledSkillSchema>;
export type InstalledJson = z.infer<typeof InstalledJsonSchema>;
