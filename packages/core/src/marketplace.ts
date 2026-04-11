import type { Tap, TapSkill } from "./schemas/tap";
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
 * Adapt a parsed marketplace.json into a skilltap Tap object.
 *
 * @param marketplace - Parsed marketplace data
 * @param tapUrl - The git URL of the marketplace repo (used to resolve relative paths)
 */
export function adaptMarketplaceToTap(marketplace: Marketplace, tapUrl: string): Tap {
  const seenNames = new Set<string>();
  const skills: TapSkill[] = [];

  for (const plugin of marketplace.plugins) {
    if (seenNames.has(plugin.name)) continue;

    const repo = marketplaceSourceToRepo(plugin.source, tapUrl);
    if (repo === null) continue;

    seenNames.add(plugin.name);
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
  };
}
