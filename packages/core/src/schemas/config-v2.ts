import { z } from "zod/v4";
import {
  TelemetryConfigSchema,
  UpdatesConfigSchema,
} from "./config";

// v2.0 simplification of [security].
// Three keys: scan, on_warn, trust.
// Drops [security.human]/[security.agent] split, presets, and
// [[security.overrides]]. Same security policy regardless of --agent.

export const SECURITY_SCAN_V2 = ["semantic", "static", "none"] as const;
export const SECURITY_ON_WARN_V2 = ["prompt", "fail", "install"] as const;
export const SCOPE_V2 = ["", "global", "project"] as const;

export type SecurityScanV2 = (typeof SECURITY_SCAN_V2)[number];
export type SecurityOnWarnV2 = (typeof SECURITY_ON_WARN_V2)[number];
export type ScopeV2 = (typeof SCOPE_V2)[number];

export const SecurityConfigV2Schema = z
  .object({
    scan: z.enum(SECURITY_SCAN_V2).default("static"),
    on_warn: z.enum(SECURITY_ON_WARN_V2).default("install"),
    // Glob patterns matched against tap name OR full source URL.
    // A matching source skips the scan entirely.
    trust: z.array(z.string()).default([]),
  })
  .prefault({});

// Replaces the v1.0 [agent-mode] block.
// `default = true` makes --agent the default for every invocation.
// `block = true` causes the CLI to refuse --agent (forces interactive).
export const AgentConfigSchema = z
  .object({
    default: z.boolean().default(false),
    block: z.boolean().default(false),
  })
  .prefault({});

export const ConfigV2DefaultsSchema = z
  .object({
    also: z.array(z.string()).default([]),
    scope: z.enum(SCOPE_V2).default(""),
  })
  .prefault({});

export const ConfigV2TapEntrySchema = z.object({
  name: z.string(),
  url: z.string(),
});

export const ConfigV2Schema = z.object({
  defaults: ConfigV2DefaultsSchema,
  agent: AgentConfigSchema,
  security: SecurityConfigV2Schema,
  taps: z.array(ConfigV2TapEntrySchema).default([]),
  updates: UpdatesConfigSchema.prefault({}),
  telemetry: TelemetryConfigSchema.prefault({}),
  builtin_tap: z.boolean().default(true),
  verbose: z.boolean().default(true),
  default_git_host: z.string().default("https://github.com"),
});

export type SecurityConfigV2 = z.infer<typeof SecurityConfigV2Schema>;
export type AgentConfig = z.infer<typeof AgentConfigSchema>;
export type ConfigV2Defaults = z.infer<typeof ConfigV2DefaultsSchema>;
export type ConfigV2TapEntry = z.infer<typeof ConfigV2TapEntrySchema>;
export type ConfigV2 = z.infer<typeof ConfigV2Schema>;
