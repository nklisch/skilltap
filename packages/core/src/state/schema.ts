import { z } from "zod/v4";
import { InstalledSkillSchema } from "../schemas/installed";
import { PluginRecordSchema } from "../schemas/plugins";

// Standalone MCP server installs — used when a user runs
// `skilltap install mcp:<source>` to install only an MCP server, not
// a full plugin.
export const StoredMcpStdioConfigSchema = z.object({
  type: z.literal("stdio").default("stdio"),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const StoredMcpHttpConfigSchema = z.object({
  type: z.literal("http"),
  url: z.string(),
  headers: z.record(z.string(), z.string()).default({}),
});

export const StoredMcpStandaloneSchema = z.object({
  name: z.string(),
  source: z.string(),
  config: z.union([StoredMcpStdioConfigSchema, StoredMcpHttpConfigSchema]),
  targets: z.array(z.string()).default([]),
  installedAt: z.iso.datetime(),
});

// Unified state file for v2.0 — replaces installed.json + plugins.json.
// Lives at ~/.config/skilltap/state.json (global) and
// <projectRoot>/.agents/state.json (project).
export const StateSchema = z.object({
  version: z.literal(2),
  skills: z.array(InstalledSkillSchema).default([]),
  plugins: z.array(PluginRecordSchema).default([]),
  mcpServers: z.array(StoredMcpStandaloneSchema).default([]),
});

export type StoredMcpStdioConfig = z.infer<typeof StoredMcpStdioConfigSchema>;
export type StoredMcpHttpConfig = z.infer<typeof StoredMcpHttpConfigSchema>;
export type StoredMcpStandalone = z.infer<typeof StoredMcpStandaloneSchema>;
export type State = z.infer<typeof StateSchema>;
