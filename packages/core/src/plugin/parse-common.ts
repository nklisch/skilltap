import { relative, resolve } from "node:path";
import { scan } from "../scanner";
import type { PluginManifest } from "../schemas/plugin";

export async function discoverSkills(
  pluginDir: string,
  skillPaths?: string | string[],
): Promise<PluginManifest["components"]> {
  const components: PluginManifest["components"] = [];
  const paths = skillPaths
    ? (Array.isArray(skillPaths) ? skillPaths : [skillPaths])
    : [pluginDir];

  for (const p of paths) {
    const absDir = p === pluginDir ? pluginDir : resolve(pluginDir, p);
    let skills: Awaited<ReturnType<typeof scan>> = [];
    try {
      skills = await scan(absDir);
    } catch {
      // Path override points to non-existent directory — treat as no skills
    }
    for (const skill of skills) {
      components.push({
        type: "skill",
        name: skill.name,
        path: relative(pluginDir, skill.path),
        description: skill.description,
      });
    }
  }
  return components;
}
