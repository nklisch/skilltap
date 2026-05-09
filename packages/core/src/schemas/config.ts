import { z } from "zod/v4";

// ---------------------------------------------------------------------------
// Enum value constants — single source of truth for all valid enum options.
// Import these wherever enum values need to be listed (config-keys, wizards, etc.)
// ---------------------------------------------------------------------------

export const SCAN_MODES = ["semantic", "static", "none"] as const;
export const ON_WARN_MODES = ["prompt", "fail", "install"] as const;
export const SCOPE_VALUES = ["", "global", "project"] as const;
export const AUTO_UPDATE_MODES = ["off", "patch", "minor"] as const;
export const SHOW_DIFF_MODES = ["full", "stat", "none"] as const;
export const SOURCE_TYPES = ["tap", "git", "npm", "local"] as const;

// ---------------------------------------------------------------------------
// [security] — policy. Three keys.
// ---------------------------------------------------------------------------

export const SecurityConfigSchema = z
  .object({
    scan: z.enum(SCAN_MODES).default("static"),
    on_warn: z.enum(ON_WARN_MODES).default("install"),
    trust: z.array(z.string()).default([]),
  })
  .prefault({});

// ---------------------------------------------------------------------------
// [scanner] — operational config. What CLI to invoke, size limits, etc.
// ---------------------------------------------------------------------------

export const ScannerConfigSchema = z
  .object({
    agent_cli: z.string().default(""),
    ollama_model: z.string().default(""),
    threshold: z.number().int().min(0).max(10).default(5),
    max_size: z.number().int().default(51200),
  })
  .prefault({});

export const UpdatesConfigSchema = z
  .object({
    auto_update: z.enum(AUTO_UPDATE_MODES).default("off"),
    interval_hours: z.number().int().default(24),
    skill_check_interval_hours: z.number().int().default(24),
    show_diff: z.enum(SHOW_DIFF_MODES).default("full"),
  })
  .prefault({});

export const TelemetryConfigSchema = z
  .object({
    enabled: z.boolean().default(false),
    notice_shown: z.boolean().default(false),
    anonymous_id: z.string().default(""),
  })
  .prefault({});

export const RegistrySourceSchema = z.object({
  name: z.string(),
  url: z.string(),
});

export const RegistryConfigSchema = z
  .object({
    enabled: z.array(z.string()).default(["skills.sh"]),
    sources: z.array(RegistrySourceSchema).default([]),
  })
  .prefault({});

export const ConfigDefaultsSchema = z
  .object({
    also: z.array(z.string()).default([]),
    yes: z.boolean().default(false),
    scope: z.enum(SCOPE_VALUES).default(""),
  })
  .prefault({});

export const TapEntrySchema = z.object({
  name: z.string(),
  url: z.string(),
  type: z.enum(["git", "http"]).default("git"),
});

export const ConfigSchema = z.object({
  defaults: ConfigDefaultsSchema,
  security: SecurityConfigSchema,
  scanner: ScannerConfigSchema,
  registry: RegistryConfigSchema,
  taps: z.array(TapEntrySchema).default([]),
  updates: UpdatesConfigSchema,
  telemetry: TelemetryConfigSchema,
  builtin_tap: z.boolean().default(true),
  verbose: z.boolean().default(true),
  default_git_host: z.string().default("https://github.com"),
});

export type SecurityConfig = z.infer<typeof SecurityConfigSchema>;
export type ScannerConfig = z.infer<typeof ScannerConfigSchema>;
export type UpdatesConfig = z.infer<typeof UpdatesConfigSchema>;
export type TelemetryConfig = z.infer<typeof TelemetryConfigSchema>;
export type RegistryConfig = z.infer<typeof RegistryConfigSchema>;
export type RegistrySource = z.infer<typeof RegistrySourceSchema>;
export type ConfigDefaults = z.infer<typeof ConfigDefaultsSchema>;
export type TapEntry = z.infer<typeof TapEntrySchema>;
export type Config = z.infer<typeof ConfigSchema>;
