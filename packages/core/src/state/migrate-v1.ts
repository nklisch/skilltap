import type {
  LegacyInstalledJson,
  LegacyPluginsJson,
} from "../migrate/legacy-schemas";
import type { State } from "./schema";

// Pure structural merge: InstalledSkill + PluginRecord schemas are reused
// unchanged in StateSchema, so no per-field translation is needed.
// Standalone MCP servers (`mcp:` prefix installs) start empty after migration
// — only `install mcp` populates that array.
export function migrateV1State(
  installed: LegacyInstalledJson,
  plugins: LegacyPluginsJson,
): State {
  return {
    version: 2,
    skills: installed.skills,
    plugins: plugins.plugins,
    mcpServers: [],
  };
}
