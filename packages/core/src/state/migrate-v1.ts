import type { InstalledJson } from "../schemas/installed";
import type { PluginsJson } from "../schemas/plugins";
import type { State } from "./schema";

// Pure structural merge: InstalledSkill + PluginRecord schemas are reused
// unchanged in StateSchema, so no per-field translation is needed.
// Standalone MCP servers (`mcp:` prefix installs) start empty — Phase 35
// adds installs that populate this array.
export function migrateV1State(
  installed: InstalledJson,
  plugins: PluginsJson,
): State {
  return {
    version: 2,
    skills: installed.skills,
    plugins: plugins.plugins,
    mcpServers: [],
  };
}
