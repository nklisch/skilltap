import { z } from "zod/v4";

export const SecurityConfigSchema = z.object({
  scan: z.enum(["static", "semantic", "off"]).default("static"),
  on_warn: z.enum(["prompt", "fail"]).default("prompt"),
  require_scan: z.boolean().default(false),
  agent: z.string().default(""),
  threshold: z.number().int().min(0).max(10).default(5),
  max_size: z.number().int().default(51200),
  ollama_model: z.string().default(""),
});

export const AgentModeSchema = z.object({
  enabled: z.boolean().default(false),
  scope: z.enum(["global", "project"]).default("project"),
});

export const ConfigSchema = z.object({
  defaults: z
    .object({
      also: z.array(z.string()).default([]),
      yes: z.boolean().default(false),
      scope: z.enum(["global", "project", ""]).default(""),
    })
    // prefault passes {} through the schema, applying nested defaults (Zod 4 vs Zod 3's default({}))
    .prefault({}),
  security: SecurityConfigSchema.prefault({}),
  "agent-mode": AgentModeSchema.prefault({}),
  taps: z
    .array(
      z.object({
        name: z.string(),
        url: z.string(),
      }),
    )
    .default([]),
});

export type SecurityConfig = z.infer<typeof SecurityConfigSchema>;
export type AgentMode = z.infer<typeof AgentModeSchema>;
export type Config = z.infer<typeof ConfigSchema>;
