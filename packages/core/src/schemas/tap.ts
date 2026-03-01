import { z } from "zod/v4";

export const TapSkillSchema = z.object({
  name: z.string(),
  description: z.string(),
  repo: z.string(),
  tags: z.array(z.string()).default([]),
});

export const TapSchema = z.object({
  name: z.string(),
  description: z.string().optional(),
  skills: z.array(TapSkillSchema),
});

export type TapSkill = z.infer<typeof TapSkillSchema>;
export type Tap = z.infer<typeof TapSchema>;
