import { homedir } from "node:os";
import { join } from "node:path";
import { z } from "zod/v4";
import { detectPlugin } from "../plugin/detect";
import { err, ok, type Result, UserError } from "../types";
import type { AgentPluginScanner, DiscoveredAgentPlugin } from "./types";

// --- Schemas (tolerant) ---

const InstalledPluginEntrySchema = z
  .object({
    scope: z.enum(["user", "local"]),
    projectPath: z.string().optional(),
    installPath: z.string(),
    version: z.string(),
    installedAt: z.string(),
    lastUpdated: z.string(),
    gitCommitSha: z.string().optional(),
  })
  .passthrough();

const InstalledPluginsFileSchema = z
  .object({
    version: z.literal(2),
    plugins: z.record(z.string(), z.array(InstalledPluginEntrySchema)),
  })
  .passthrough();

const MarketplaceSourceSchema = z
  .object({
    source: z.string(), // "github" | "local" | "url" — tolerant
    repo: z.string().optional(),
    url: z.string().optional(),
    path: z.string().optional(),
  })
  .passthrough();

const KnownMarketplaceEntrySchema = z
  .object({
    source: MarketplaceSourceSchema,
    installLocation: z.string(),
    lastUpdated: z.string(),
    autoUpdate: z.boolean().optional(),
  })
  .passthrough();

const KnownMarketplacesFileSchema = z.record(
  z.string(),
  KnownMarketplaceEntrySchema,
);

// --- Path helpers ---

export function claudePluginsDir(overrideEnv?: string): string {
  const base = overrideEnv ?? process.env.CLAUDE_CONFIG_HOME ?? homedir();
  // When CLAUDE_CONFIG_HOME is set, it IS the .claude dir equivalent
  if (overrideEnv || process.env.CLAUDE_CONFIG_HOME) {
    return join(base, "plugins");
  }
  return join(base, ".claude", "plugins");
}

function installedPluginsPath(overrideEnv?: string): string {
  return join(claudePluginsDir(overrideEnv), "installed_plugins.json");
}

function knownMarketplacesPath(overrideEnv?: string): string {
  return join(claudePluginsDir(overrideEnv), "known_marketplaces.json");
}

// --- Scanner ---

export function createClaudeCodeScanner(
  overrideEnv?: string,
): AgentPluginScanner {
  return {
    name: "claude-code",
    async detect(): Promise<boolean> {
      return await Bun.file(installedPluginsPath(overrideEnv)).exists();
    },
    async scan(): Promise<Result<DiscoveredAgentPlugin[], UserError>> {
      const installedRaw = await readJsonTolerant(
        installedPluginsPath(overrideEnv),
        InstalledPluginsFileSchema,
      );
      if (!installedRaw.ok) return installedRaw;
      const installed = installedRaw.value;

      const marketplacesRaw = await readJsonTolerant(
        knownMarketplacesPath(overrideEnv),
        KnownMarketplacesFileSchema,
      );
      // Marketplaces file is OPTIONAL — adoption still works without it,
      // just with less source-canonical metadata.
      const marketplaces = marketplacesRaw.ok ? marketplacesRaw.value : {};

      const results: DiscoveredAgentPlugin[] = [];
      for (const [key, entries] of Object.entries(installed.plugins)) {
        // key is "<name>@<marketplace>". Use lastIndexOf to handle plugin names with @.
        const at = key.lastIndexOf("@");
        if (at < 0) continue; // malformed; skip
        const name = key.slice(0, at);
        const marketplaceName = key.slice(at + 1);

        const marketplace = marketplaces[marketplaceName];
        const sourceUrl = marketplaceToSourceUrl(marketplace);

        for (const entry of entries) {
          const manifestResult = await detectPlugin(entry.installPath);
          if (!manifestResult.ok || manifestResult.value === null) {
            // Cache is stale / format doesn't match; skip silently.
            continue;
          }
          const manifest = manifestResult.value;

          results.push({
            scannerName: "claude-code",
            name,
            marketplaceName,
            sourceUrl,
            installPath: entry.installPath,
            version: entry.version,
            sha: entry.gitCommitSha ?? null,
            // Map Claude Code's scope vocabulary to skilltap's:
            //   user  → global
            //   local → project (uses entry.projectPath)
            scope: entry.scope === "user" ? "global" : "project",
            projectRoot:
              entry.scope === "local" ? entry.projectPath : undefined,
            installedAt: entry.installedAt,
            updatedAt: entry.lastUpdated,
            manifest,
          });
        }
      }
      return ok(results);
    },
  };
}

function marketplaceToSourceUrl(
  m: z.infer<typeof KnownMarketplaceEntrySchema> | undefined,
): string | null {
  if (!m) return null;
  const s = m.source;
  if (s.source === "github" && s.repo) {
    return `github:${s.repo}`;
  }
  if (s.url) return s.url;
  if (s.path) return s.path;
  return null;
}

async function readJsonTolerant<T extends z.ZodTypeAny>(
  path: string,
  schema: T,
): Promise<Result<z.infer<T>, UserError>> {
  const file = Bun.file(path);
  if (!(await file.exists())) {
    return err(new UserError(`File not found: ${path}`));
  }
  let raw: unknown;
  try {
    raw = await file.json();
  } catch (e) {
    return err(new UserError(`Failed to parse JSON at ${path}: ${e}`));
  }
  const parsed = schema.safeParse(raw);
  if (!parsed.success) {
    return err(
      new UserError(
        `Schema mismatch at ${path}: ${parsed.error.issues
          .slice(0, 3)
          .map((i) => i.message)
          .join("; ")}`,
        "Claude Code's plugin file format may have changed. Run `skilltap doctor` for details.",
      ),
    );
  }
  return ok(parsed.data);
}
