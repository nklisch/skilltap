import { z } from "zod/v4";

export const InstalledSkillSchema = z.object({
  name: z.string(),
  repo: z.string().nullable(),
  ref: z.string().nullable(),
  sha: z.string().nullable(),
  scope: z.enum(["global", "project", "linked"]),
  path: z.string().nullable(),
  tap: z.string().nullable(),
  also: z.array(z.string()),
  installedAt: z.iso.datetime(),
  updatedAt: z.iso.datetime(),
});

export const InstalledJsonSchema = z.object({
  version: z.literal(1),
  skills: z.array(InstalledSkillSchema),
});

export type InstalledSkill = z.infer<typeof InstalledSkillSchema>;
export type InstalledJson = z.infer<typeof InstalledJsonSchema>;
