import { z } from "zod/v4";

// ---------------------------------------------------------------------------
// Enum value constants — single source of truth for all valid enum options.
// Import these wherever enum values need to be listed (config-keys, wizards, etc.)
// ---------------------------------------------------------------------------

export const SCAN_MODES = ["static", "semantic", "off"] as const;
export const ON_WARN_MODES = ["prompt", "fail"] as const;
export const AGENT_MODE_SCOPES = ["global", "project"] as const;
export const SCOPE_VALUES = ["", "global", "project"] as const;
export const AUTO_UPDATE_MODES = ["off", "patch", "minor"] as const;
export const SHOW_DIFF_MODES = ["full", "stat", "none"] as const;

export const SecurityConfigSchema = z.object({
  scan: z.enum(SCAN_MODES).default("static"),
  on_warn: z.enum(ON_WARN_MODES).default("prompt"),
  require_scan: z.boolean().default(false),
  agent: z.string().default(""),
  threshold: z.number().int().min(0).max(10).default(5),
  max_size: z.number().int().default(51200),
  ollama_model: z.string().default(""),
});

export const AgentModeSchema = z.object({
  enabled: z.boolean().default(false),
  scope: z.enum(AGENT_MODE_SCOPES).default("project"),
});

export const UpdatesConfigSchema = z.object({
  // "off" = notify only, "patch" = auto-update patch releases, "minor" = auto-update patch+minor
  auto_update: z.enum(AUTO_UPDATE_MODES).default("off"),
  interval_hours: z.number().int().default(24),
  skill_check_interval_hours: z.number().int().default(24),
  // "full" = coloured unified diff, "stat" = file-level counts only, "none" = hide until confirm
  show_diff: z.enum(SHOW_DIFF_MODES).default("full"),
});

export const TelemetryConfigSchema = z.object({
  enabled: z.boolean().default(false),
  notice_shown: z.boolean().default(false),
  anonymous_id: z.string().default(""),
});

export const RegistrySourceSchema = z.object({
  name: z.string(),
  url: z.string(),
});

export const RegistryConfigSchema = z.object({
  /** Which registries to search (in order). Built-in: "skills.sh". */
  enabled: z.array(z.string()).default(["skills.sh"]),
  /** Custom registry sources implementing the skills.sh search API. */
  sources: z.array(RegistrySourceSchema).default([]),
  /** @deprecated Use enabled = [] to disable all registries instead. */
  allow_npm: z.boolean().default(true),
}).prefault({});

export const ConfigSchema = z.object({
  defaults: z
    .object({
      also: z.array(z.string()).default([]),
      yes: z.boolean().default(false),
      scope: z.enum(SCOPE_VALUES).default(""),
    })
    // prefault passes {} through the schema, applying nested defaults (Zod 4 vs Zod 3's default({}))
    .prefault({}),
  security: SecurityConfigSchema.prefault({}),
  "agent-mode": AgentModeSchema.prefault({}),
  registry: RegistryConfigSchema,
  /** Whether the built-in skilltap-skills tap is enabled. Set to false to opt out. */
  builtin_tap: z.boolean().default(true),
  /** Show install step details (clone, scan, placement). Disable with verbose = false or --no-verbose. */
  verbose: z.boolean().default(true),
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
  /** Default git host for owner/repo shorthand. Defaults to "https://github.com". */
  default_git_host: z.string().default("https://github.com"),
});

export type SecurityConfig = z.infer<typeof SecurityConfigSchema>;
export type AgentMode = z.infer<typeof AgentModeSchema>;
export type UpdatesConfig = z.infer<typeof UpdatesConfigSchema>;
export type TelemetryConfig = z.infer<typeof TelemetryConfigSchema>;
export type RegistryConfig = z.infer<typeof RegistryConfigSchema>;
export type RegistrySource = z.infer<typeof RegistrySourceSchema>;
export type Config = z.infer<typeof ConfigSchema>;
