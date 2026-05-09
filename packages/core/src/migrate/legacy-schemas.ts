import { z } from "zod/v4";
import { InstalledSkillSchema } from "../schemas/installed";
import { PluginRecordSchema } from "../schemas/plugins";

// Legacy file-wrapper schemas. Used only when reading pre-v2 installed.json /
// plugins.json files. Production state lives in state.json.

export const LegacyInstalledJsonSchema = z.object({
  version: z.literal(1),
  skills: z.array(InstalledSkillSchema),
});
export type LegacyInstalledJson = z.infer<typeof LegacyInstalledJsonSchema>;

export const LegacyPluginsJsonSchema = z.object({
  version: z.literal(1),
  plugins: z.array(PluginRecordSchema).default([]),
});
export type LegacyPluginsJson = z.infer<typeof LegacyPluginsJsonSchema>;
