import { z } from "zod/v4";

// Native v2.0 plugin manifest format. Lives at .skilltap/<plugin-name>.toml
// in a publishing repo. Read by detect.ts alongside .claude-plugin/plugin.json
// and .codex-plugin/plugin.json (existing v1.0 input formats).

export const PluginV2SkillSchema = z.object({
  name: z.string().regex(/^[a-z0-9]+(-[a-z0-9]+)*$/),
  path: z.string(),
  description: z.string().default(""),
});

export const PluginV2StdioServerSchema = z.object({
  type: z.literal("stdio").default("stdio"),
  name: z.string(),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const PluginV2HttpServerSchema = z.object({
  type: z.literal("http"),
  name: z.string(),
  url: z.string(),
  headers: z.record(z.string(), z.string()).default({}),
});

export const PluginV2ServerSchema = z.union([
  PluginV2StdioServerSchema,
  PluginV2HttpServerSchema,
]);

export const PluginV2AgentSchema = z.object({
  name: z.string(),
  path: z.string(),
});

export const PluginManifestV2Schema = z.object({
  name: z.string().regex(/^[a-z0-9]+(-[a-z0-9]+)*$/),
  version: z.string(),
  description: z.string().default(""),
  // publish defaults to false — explicit opt-in for outside installation.
  // Repos with publish=false (or omitted) can still be installed for their
  // consumer-side dependencies; the plugin part is just not exposed.
  publish: z.boolean().default(false),
  skills: z.array(PluginV2SkillSchema).default([]),
  servers: z.array(PluginV2ServerSchema).default([]),
  agents: z.array(PluginV2AgentSchema).default([]),
});

export type PluginV2Skill = z.infer<typeof PluginV2SkillSchema>;
export type PluginV2StdioServer = z.infer<typeof PluginV2StdioServerSchema>;
export type PluginV2HttpServer = z.infer<typeof PluginV2HttpServerSchema>;
export type PluginV2Server = z.infer<typeof PluginV2ServerSchema>;
export type PluginV2Agent = z.infer<typeof PluginV2AgentSchema>;
export type PluginManifestV2 = z.infer<typeof PluginManifestV2Schema>;
