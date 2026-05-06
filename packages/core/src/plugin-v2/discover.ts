import { discoverPublishablePlugins } from "../manifest/publish";
import type { PluginManifest } from "../schemas/plugin";
import { ok, type Result, UserError } from "../types";
import { pluginV2ToManifest } from "./normalize";

// Discover all publishable v2.0 plugins in a repo and normalize them to
// the internal PluginManifest type. Rejected files (publish=false, invalid)
// are surfaced separately so callers can warn or ignore.
export interface SkilltapDiscovery {
  manifests: PluginManifest[];
  rejected: { path: string; reason: string }[];
}

export async function discoverSkilltapPlugins(
  repoRoot: string,
): Promise<Result<SkilltapDiscovery, UserError>> {
  const found = await discoverPublishablePlugins(repoRoot);

  const manifests: PluginManifest[] = [];
  for (const v2 of found.publishable) {
    const result = await pluginV2ToManifest(v2, repoRoot);
    if (!result.ok) return result;
    manifests.push(result.value);
  }

  return ok({
    manifests,
    rejected: found.rejected,
  });
}
