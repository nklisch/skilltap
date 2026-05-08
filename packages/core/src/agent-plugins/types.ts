import type { PluginManifest } from "../schemas/plugin";
import type { Result, UserError } from "../types";

export interface DiscoveredAgentPlugin {
  /** Scanner that produced this record (e.g., "claude-code"). */
  scannerName: string;
  /** Plugin name from the manifest. */
  name: string;
  /** Marketplace name (claude-code: from "<name>@<marketplace>" key). Optional for non-marketplace scanners. */
  marketplaceName?: string;
  /** Canonical source URL (e.g., "github:owner/repo"). May be null when unknown. */
  sourceUrl: string | null;
  /** Absolute path to the plugin's content (skilltap reads from here, doesn't copy). */
  installPath: string;
  /** Plugin version. */
  version: string;
  /** Git SHA at install time. May be null. */
  sha: string | null;
  /** Skilltap scope this should be adopted into. */
  scope: "global" | "project";
  /** Project root if scope === "project". */
  projectRoot?: string;
  installedAt: string;
  updatedAt: string;
  /** Parsed plugin manifest from installPath. */
  manifest: PluginManifest;
}

export interface AgentPluginScanner {
  /** Identifier for this scanner (e.g., "claude-code", "codex"). */
  name: string;
  /** Returns true if this agent's plugin system is detectable on the host. */
  detect(): Promise<boolean>;
  /** Returns the agent's installed plugins, or an empty array if none. */
  scan(): Promise<Result<DiscoveredAgentPlugin[], UserError>>;
}
