/**
 * Plugin state schemas.
 *
 * `Stored*ComponentSchema`, `PluginRecordSchema` / `PluginRecord` — the
 * canonical record shapes for installed plugins and their components.
 * `state.json`'s `plugins[]` array uses `PluginRecord`, and every consumer
 * (plugin install, lifecycle, status, doctor checks) references these types.
 */
import { z } from "zod/v4";
import { DEFAULT_AGENT_ID } from "../symlink";
import { PLUGIN_FORMATS } from "./plugin";

export const StoredSkillComponentSchema = z.object({
  type: z.literal("skill"),
  name: z.string(),
  active: z.boolean().default(true),
});

export const StoredMcpStdioComponentSchema = z.object({
  type: z.literal("mcp"),
  serverType: z.literal("stdio").default("stdio"),
  name: z.string(),
  active: z.boolean().default(true),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const StoredMcpHttpComponentSchema = z.object({
  type: z.literal("mcp"),
  serverType: z.literal("http"),
  name: z.string(),
  active: z.boolean().default(true),
  url: z.string(),
  headers: z.record(z.string(), z.string()).default({}),
});

export const StoredMcpComponentSchema = z.union([
  StoredMcpStdioComponentSchema,
  StoredMcpHttpComponentSchema,
]);

export const StoredAgentComponentSchema = z.object({
  type: z.literal("agent"),
  name: z.string(),
  active: z.boolean().default(true),
  platform: z.string().default(DEFAULT_AGENT_ID),
});

export const StoredComponentSchema = z.union([
  StoredSkillComponentSchema,
  StoredMcpStdioComponentSchema,
  StoredMcpHttpComponentSchema,
  StoredAgentComponentSchema,
]);

export const PluginRecordSchema = z.object({
  name: z.string(),
  description: z.string().default(""),
  format: z.enum(PLUGIN_FORMATS),
  repo: z.string().nullable(),
  ref: z.string().nullable(),
  sha: z.string().nullable(),
  scope: z.enum(["global", "project"]),
  /** Explicit on-disk path. Used for adopted plugins that point at an external cache (e.g., Claude Code's plugin cache). Null for normally-installed plugins. */
  path: z.string().nullable().default(null),
  also: z.array(z.string()).default([]),
  tap: z.string().nullable().default(null),
  components: z.array(StoredComponentSchema),
  installedAt: z.iso.datetime(),
  updatedAt: z.iso.datetime(),
  active: z.boolean().default(true),
});

export type StoredSkillComponent = z.infer<typeof StoredSkillComponentSchema>;
export type StoredMcpStdioComponent = z.infer<
  typeof StoredMcpStdioComponentSchema
>;
export type StoredMcpHttpComponent = z.infer<
  typeof StoredMcpHttpComponentSchema
>;
export type StoredMcpComponent = z.infer<typeof StoredMcpComponentSchema>;
export type StoredAgentComponent = z.infer<typeof StoredAgentComponentSchema>;
export type StoredComponent = z.infer<typeof StoredComponentSchema>;
export type PluginRecord = z.infer<typeof PluginRecordSchema>;
