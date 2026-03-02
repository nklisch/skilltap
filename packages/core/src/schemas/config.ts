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

export const UpdatesConfigSchema = z.object({
  // "off" = notify only, "patch" = auto-update patch releases, "minor" = auto-update patch+minor
  auto_update: z.enum(["off", "patch", "minor"]).default("off"),
  interval_hours: z.number().int().default(24),
});

export const RegistryConfigSchema = z.object({
  allow_npm: z.boolean().default(true),
});

export const TelemetryConfigSchema = z.object({
  enabled: z.boolean().default(false),
  notice_shown: z.boolean().default(false),
  anonymous_id: z.string().default(""),
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
  registry: RegistryConfigSchema.prefault({}),
  taps: z
    .array(
      z.object({
        name: z.string(),
        url: z.string(),
        type: z.enum(["git", "http"]).default("git"),
        auth_token: z.string().optional(),
        auth_env: z.string().optional(),
      }),
    )
    .default([]),
  updates: UpdatesConfigSchema.prefault({}),
  telemetry: TelemetryConfigSchema.prefault({}),
});

export type SecurityConfig = z.infer<typeof SecurityConfigSchema>;
export type AgentMode = z.infer<typeof AgentModeSchema>;
export type UpdatesConfig = z.infer<typeof UpdatesConfigSchema>;
export type RegistryConfig = z.infer<typeof RegistryConfigSchema>;
export type TelemetryConfig = z.infer<typeof TelemetryConfigSchema>;
export type Config = z.infer<typeof ConfigSchema>;
