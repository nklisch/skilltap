import { join } from "node:path";
import { detectPlugin } from "./plugin/detect";
import type { Tap, TapPlugin, TapSkill } from "./schemas/tap";
import type { Marketplace, MarketplacePluginSource } from "./schemas/marketplace";

/**
 * Convert a marketplace plugin source to a TapSkill.repo string
 * that the source adapter chain can resolve.
 *
 * @param source - The plugin source from marketplace.json
 * @param tapUrl - The git URL of the marketplace repo itself (for relative paths)
 */
export function marketplaceSourceToRepo(
  source: MarketplacePluginSource,
  tapUrl: string,
): string | null {
  if (typeof source === "string") {
    // Relative path — the marketplace repo itself contains the skills
    return tapUrl;
  }
  switch (source.source) {
    case "github":
      return source.repo;
    case "url":
      return source.url;
    case "git-subdir":
      // Path not preserved — documented limitation
      return source.url;
    case "npm":
      return `npm:${source.package}`;
    default:
      return null;
  }
}

/**
 * Convert a detected PluginManifest into a TapPlugin entry.
 * Adjusts component paths to be relative to the tap repo root.
 */
function manifestToTapPlugin(
  manifest: NonNullable<Awaited<ReturnType<typeof detectPlugin>> extends { ok: true; value: infer V } ? V : never>,
  marketplacePlugin: { name: string; description?: string; tags?: string[]; category?: string; version?: string },
  sourcePrefix: string,
): TapPlugin {
  const skills = manifest.components
    .filter((c): c is Extract<typeof c, { type: "skill" }> => c.type === "skill")
    .map((c) => ({
      name: c.name,
      path: join(sourcePrefix, c.path),
      description: c.description ?? "",
    }));

  const agents = manifest.components
    .filter((c): c is Extract<typeof c, { type: "agent" }> => c.type === "agent")
    .map((c) => ({
      name: c.name,
      path: join(sourcePrefix, c.path),
    }));

  // Build inline mcpServers object from MCP components
  const mcpComponents = manifest.components.filter(
    (c): c is Extract<typeof c, { type: "mcp" }> => c.type === "mcp",
  );
  let mcpServers: TapPlugin["mcpServers"];
  if (mcpComponents.length > 0) {
    const servers: Record<string, unknown> = {};
    for (const c of mcpComponents) {
      const { server } = c;
      if (server.type === "stdio") {
        const entry: Record<string, unknown> = { command: server.command };
        if (server.args.length > 0) entry.args = server.args;
        if (Object.keys(server.env).length > 0) entry.env = server.env;
        servers[server.name] = entry;
      } else {
        servers[server.name] = { type: "http", url: server.url };
      }
    }
    mcpServers = servers;
  }

  return {
    name: marketplacePlugin.name,
    description: marketplacePlugin.description ?? "",
    version: marketplacePlugin.version,
    skills,
    mcpServers,
    agents,
    tags: marketplacePlugin.tags ?? (marketplacePlugin.category ? [marketplacePlugin.category] : []),
  };
}

/**
 * Adapt a parsed marketplace.json into a skilltap Tap object.
 *
 * For plugins with relative-path sources that contain .claude-plugin/plugin.json,
 * produces TapPlugin entries (with full skill/MCP/agent components). Otherwise
 * produces TapSkill entries (skill-only, as before).
 *
 * @param marketplace - Parsed marketplace data
 * @param tapUrl - The git URL of the marketplace repo (used to resolve relative paths)
 * @param tapDir - Local directory of the cloned tap (for detecting plugin.json in relative-path sources)
 */
export async function adaptMarketplaceToTap(
  marketplace: Marketplace,
  tapUrl: string,
  tapDir?: string,
): Promise<Tap> {
  const seenNames = new Set<string>();
  const skills: TapSkill[] = [];
  const plugins: TapPlugin[] = [];

  for (const plugin of marketplace.plugins) {
    if (seenNames.has(plugin.name)) continue;

    const repo = marketplaceSourceToRepo(plugin.source, tapUrl);
    if (repo === null) continue;

    seenNames.add(plugin.name);

    // For relative-path sources in a local tap dir, check for plugin.json
    if (tapDir && typeof plugin.source === "string") {
      const sourcePrefix = plugin.source.replace(/^\.\//, "");
      const pluginDir = join(tapDir, sourcePrefix);
      const detected = await detectPlugin(pluginDir);
      if (detected.ok && detected.value) {
        plugins.push(manifestToTapPlugin(detected.value, plugin, sourcePrefix));
        continue;
      }
      // No plugin.json detected — fall through to skill entry
    }

    skills.push({
      name: plugin.name,
      description: plugin.description ?? `Plugin from ${marketplace.name} marketplace`,
      repo,
      tags: plugin.tags ?? (plugin.category ? [plugin.category] : []),
      plugin: true,
    });
  }

  return {
    name: marketplace.name,
    description: marketplace.metadata?.description,
    skills,
    plugins,
  };
}
