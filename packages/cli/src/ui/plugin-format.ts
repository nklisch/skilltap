import {
  loadPlugins,
  type PluginManifest,
  type PluginRecord,
  type StoredComponent,
} from "@skilltap/core";

/** One-line "type: name" label for a stored component. Used in CLI output. */
export function componentLabel(c: StoredComponent): string {
  if (c.type === "skill") return `skill: ${c.name}`;
  if (c.type === "mcp") return `mcp: ${c.name}`;
  return `agent: ${c.name}`;
}

/**
 * Look up an installed plugin by name across global + project state.
 * Returns null when not found in either scope or when state files fail to load.
 */
export async function loadPluginByName(
  name: string,
  projectRoot: string | undefined,
): Promise<PluginRecord | null> {
  const globalResult = await loadPlugins();
  if (!globalResult.ok) return null;
  const projectResult = projectRoot ? await loadPlugins(projectRoot) : null;
  const all = [
    ...globalResult.value.plugins,
    ...(projectResult?.ok ? projectResult.value.plugins : []),
  ];
  return all.find((p) => p.name === name) ?? null;
}

export function componentSummary(record: PluginRecord): string {
  const counts = { skill: 0, mcp: 0, agent: 0 };
  for (const c of record.components) counts[c.type]++;
  const parts: string[] = [];
  if (counts.skill > 0)
    parts.push(`${counts.skill} ${counts.skill === 1 ? "skill" : "skills"}`);
  if (counts.mcp > 0)
    parts.push(`${counts.mcp} ${counts.mcp === 1 ? "MCP" : "MCPs"}`);
  if (counts.agent > 0)
    parts.push(`${counts.agent} ${counts.agent === 1 ? "agent" : "agents"}`);
  return parts.join(", ") || "no components";
}

/** Summarize components from a manifest (pre-install, for detection prompt). */
export function pluginComponentSummary(manifest: PluginManifest): string {
  const counts = { skill: 0, mcp: 0, agent: 0 };
  for (const c of manifest.components) counts[c.type]++;
  const parts: string[] = [];
  if (counts.skill > 0)
    parts.push(`${counts.skill} ${counts.skill === 1 ? "skill" : "skills"}`);
  if (counts.mcp > 0)
    parts.push(`${counts.mcp} ${counts.mcp === 1 ? "MCP" : "MCPs"}`);
  if (counts.agent > 0)
    parts.push(`${counts.agent} ${counts.agent === 1 ? "agent" : "agents"}`);
  return parts.join(", ") || "no components";
}
