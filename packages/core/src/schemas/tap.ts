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

export const TapPluginSkillSchema = z.object({
  name: z.string(),
  path: z.string(),
  description: z.string().default(""),
});

export const TapPluginAgentSchema = z.object({
  name: z.string(),
  path: z.string(),
});

export const TapPluginSchema = z.object({
  name: z.string(),
  description: z.string().default(""),
  version: z.string().optional(),
  skills: z.array(TapPluginSkillSchema).default([]),
  mcpServers: z.union([
    z.string(),
    z.record(z.string(), z.unknown()),
  ]).optional(),
  agents: z.array(TapPluginAgentSchema).default([]),
  tags: z.array(z.string()).default([]),
});

export const TapSchema = z.object({
  name: z.string(),
  description: z.string().optional(),
  skills: z.array(TapSkillSchema),
  plugins: z.array(TapPluginSchema).default([]),
});

export type TapTrust = z.infer<typeof TapTrustSchema>;
export type TapSkill = z.infer<typeof TapSkillSchema>;
export type TapPluginSkill = z.infer<typeof TapPluginSkillSchema>;
export type TapPluginAgent = z.infer<typeof TapPluginAgentSchema>;
export type TapPlugin = z.infer<typeof TapPluginSchema>;
export type Tap = z.infer<typeof TapSchema>;
