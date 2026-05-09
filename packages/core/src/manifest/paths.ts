import { join } from "node:path";

export const MANIFEST_FILENAME = "skilltap.toml";
export const LOCKFILE_FILENAME = "skilltap.lock";
export const PUBLISH_DIR = ".skilltap";
export const CLAUDE_PLUGIN_DIR = ".claude-plugin";
export const CODEX_PLUGIN_DIR = ".codex-plugin";

export function manifestPath(projectRoot: string): string {
  return join(projectRoot, MANIFEST_FILENAME);
}

export function lockfilePath(projectRoot: string): string {
  return join(projectRoot, LOCKFILE_FILENAME);
}

export function publishDir(projectRoot: string): string {
  return join(projectRoot, PUBLISH_DIR);
}

export function claudePluginManifestPath(repoRoot: string): string {
  return join(repoRoot, CLAUDE_PLUGIN_DIR, "plugin.json");
}

export function codexPluginManifestPath(repoRoot: string): string {
  return join(repoRoot, CODEX_PLUGIN_DIR, "plugin.json");
}

export function marketplaceManifestPath(repoRoot: string): string {
  return join(repoRoot, CLAUDE_PLUGIN_DIR, "marketplace.json");
}
