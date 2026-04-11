import { z } from "zod/v4";

export const PLUGIN_FORMATS = ["claude-code", "codex", "skilltap"] as const;
export const PLUGIN_COMPONENT_TYPES = ["skill", "mcp", "agent"] as const;

export const PluginSkillComponentSchema = z.object({
  type: z.literal("skill"),
  name: z.string(),
  path: z.string(),
  description: z.string().default(""),
});

export const McpStdioServerSchema = z.object({
  type: z.literal("stdio").default("stdio"),
  name: z.string(),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const McpHttpServerSchema = z.object({
  type: z.literal("http"),
  name: z.string(),
  url: z.string(),
});

export const McpServerEntrySchema = z.union([
  McpStdioServerSchema,
  McpHttpServerSchema,
]);

export const PluginMcpComponentSchema = z.object({
  type: z.literal("mcp"),
  server: McpServerEntrySchema,
});

export const PluginAgentComponentSchema = z.object({
  type: z.literal("agent"),
  name: z.string(),
  path: z.string(),
  frontmatter: z.record(z.string(), z.unknown()).default({}),
});

export const PluginComponentSchema = z.discriminatedUnion("type", [
  PluginSkillComponentSchema,
  PluginMcpComponentSchema,
  PluginAgentComponentSchema,
]);

export const PluginManifestSchema = z.object({
  name: z.string(),
  version: z.string().optional(),
  description: z.string().default(""),
  format: z.enum(PLUGIN_FORMATS),
  pluginRoot: z.string(),
  components: z.array(PluginComponentSchema),
});

export const ClaudePluginJsonSchema = z.object({
  name: z.string(),
  description: z.string().optional(),
  version: z.string().optional(),
  author: z.object({
    name: z.string(),
    email: z.string().optional(),
    url: z.string().optional(),
  }).optional(),
  homepage: z.string().optional(),
  repository: z.string().optional(),
  license: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  skills: z.union([z.string(), z.array(z.string())]).optional(),
  commands: z.union([z.string(), z.array(z.string())]).optional(),
  agents: z.union([z.string(), z.array(z.string())]).optional(),
  mcpServers: z.union([z.string(), z.array(z.string()), z.record(z.string(), z.unknown())]).optional(),
  hooks: z.unknown().optional(),
  lspServers: z.unknown().optional(),
  outputStyles: z.unknown().optional(),
  channels: z.unknown().optional(),
  userConfig: z.unknown().optional(),
}).passthrough();

export const CodexPluginJsonSchema = z.object({
  name: z.string(),
  version: z.string(),
  description: z.string(),
  author: z.object({
    name: z.string(),
    email: z.string().optional(),
    url: z.string().optional(),
  }).optional(),
  homepage: z.string().optional(),
  repository: z.string().optional(),
  license: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  skills: z.string().optional(),
  mcpServers: z.string().optional(),
  apps: z.unknown().optional(),
  interface: z.unknown().optional(),
}).passthrough();

export type PluginSkillComponent = z.infer<typeof PluginSkillComponentSchema>;
export type McpStdioServer = z.infer<typeof McpStdioServerSchema>;
export type McpHttpServer = z.infer<typeof McpHttpServerSchema>;
export type McpServerEntry = z.infer<typeof McpServerEntrySchema>;
export type PluginMcpComponent = z.infer<typeof PluginMcpComponentSchema>;
export type PluginAgentComponent = z.infer<typeof PluginAgentComponentSchema>;
export type PluginComponent = z.infer<typeof PluginComponentSchema>;
export type PluginManifest = z.infer<typeof PluginManifestSchema>;
export type ClaudePluginJson = z.infer<typeof ClaudePluginJsonSchema>;
export type CodexPluginJson = z.infer<typeof CodexPluginJsonSchema>;
