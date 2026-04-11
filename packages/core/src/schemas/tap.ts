import { z } from "zod/v4";

export const TapTrustSchema = z.object({
  verified: z.boolean().default(false),
  verifiedBy: z.string().optional(),
  verifiedAt: z.string().optional(),
});

export const TapSkillSchema = z.object({
  name: z.string(),
  description: z.string(),
  repo: z.string(),
  tags: z.array(z.string()).default([]),
  trust: TapTrustSchema.optional(),
  plugin: z.boolean().default(false),
});

export const TapSchema = z.object({
  name: z.string(),
  description: z.string().optional(),
  skills: z.array(TapSkillSchema),
});

export type TapTrust = z.infer<typeof TapTrustSchema>;
export type TapSkill = z.infer<typeof TapSkillSchema>;
export type Tap = z.infer<typeof TapSchema>;
