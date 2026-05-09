import { z } from "zod/v4";

// Native skilltap plugin manifest format. Lives at .skilltap/<plugin-name>.toml
// in a publishing repo. Read by detect.ts alongside .claude-plugin/plugin.json
// and .codex-plugin/plugin.json (existing v1.0 input formats).

export const SkilltapSkillSchema = z.object({
  name: z.string().regex(/^[a-z0-9]+(-[a-z0-9]+)*$/),
  path: z.string(),
  description: z.string().default(""),
});

export const SkilltapStdioServerSchema = z.object({
  type: z.literal("stdio").default("stdio"),
  name: z.string(),
  command: z.string(),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).default({}),
});

export const SkilltapHttpServerSchema = z.object({
  type: z.literal("http"),
  name: z.string(),
  url: z.string(),
  headers: z.record(z.string(), z.string()).default({}),
});

export const SkilltapServerSchema = z.union([
  SkilltapStdioServerSchema,
  SkilltapHttpServerSchema,
]);

export const SkilltapAgentSchema = z.object({
  name: z.string(),
  path: z.string(),
});

export const SkilltapPluginManifestSchema = z.object({
  name: z.string().regex(/^[a-z0-9]+(-[a-z0-9]+)*$/),
  version: z.string(),
  description: z.string().default(""),
  // publish defaults to false — explicit opt-in for outside installation.
  // Repos with publish=false (or omitted) can still be installed for their
  // consumer-side dependencies; the plugin part is just not exposed.
  publish: z.boolean().default(false),
  skills: z.array(SkilltapSkillSchema).default([]),
  servers: z.array(SkilltapServerSchema).default([]),
  agents: z.array(SkilltapAgentSchema).default([]),
});

export type SkilltapSkill = z.infer<typeof SkilltapSkillSchema>;
export type SkilltapStdioServer = z.infer<typeof SkilltapStdioServerSchema>;
export type SkilltapHttpServer = z.infer<typeof SkilltapHttpServerSchema>;
export type SkilltapServer = z.infer<typeof SkilltapServerSchema>;
export type SkilltapAgent = z.infer<typeof SkilltapAgentSchema>;
export type SkilltapPluginManifest = z.infer<typeof SkilltapPluginManifestSchema>;
