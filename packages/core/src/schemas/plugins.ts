import { z } from "zod/v4";
import { PLUGIN_FORMATS } from "./plugin";

export const StoredSkillComponentSchema = z.object({
  type: z.literal("skill"),
  name: z.string(),
  active: z.boolean().default(true),
});

export const StoredMcpComponentSchema = z.object({
  type: z.literal("mcp"),
  name: z.string(),
  active: z.boolean().default(true),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const StoredAgentComponentSchema = z.object({
  type: z.literal("agent"),
  name: z.string(),
  active: z.boolean().default(true),
  platform: z.string().default("claude-code"),
});

export const StoredComponentSchema = z.discriminatedUnion("type", [
  StoredSkillComponentSchema,
  StoredMcpComponentSchema,
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
  also: z.array(z.string()).default([]),
  tap: z.string().nullable().default(null),
  components: z.array(StoredComponentSchema),
  installedAt: z.iso.datetime(),
  updatedAt: z.iso.datetime(),
  active: z.boolean().default(true),
});

export const PluginsJsonSchema = z.object({
  version: z.literal(1),
  plugins: z.array(PluginRecordSchema).default([]),
});

export type StoredSkillComponent = z.infer<typeof StoredSkillComponentSchema>;
export type StoredMcpComponent = z.infer<typeof StoredMcpComponentSchema>;
export type StoredAgentComponent = z.infer<typeof StoredAgentComponentSchema>;
export type StoredComponent = z.infer<typeof StoredComponentSchema>;
export type PluginRecord = z.infer<typeof PluginRecordSchema>;
export type PluginsJson = z.infer<typeof PluginsJsonSchema>;
