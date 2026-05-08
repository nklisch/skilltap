import { realpath } from "node:fs/promises";
import { join } from "node:path";
import { resolveSource } from "./adapters";
import { debug } from "./debug";
import { makeTmpDir, removeTmpDir } from "./fs";
import { clone, type GitError } from "./git";
import { detectPlugin } from "./plugin/detect";
import { parseMcpJson } from "./plugin/mcp";
import {
  injectMcpServers,
  namespaceMcpServer,
  parseNamespacedKey,
  removeMcpServers,
} from "./plugin/mcp-inject";
import type { McpServerEntry, PluginManifest } from "./schemas/plugin";
import { loadState } from "./state/load";
import { saveState } from "./state/save";
import type { State, StoredMcpStandalone } from "./state/schema";
import { err, ok, type Result, UserError } from "./types";

export interface McpInstallOptions {
  scope: "global" | "project";
  projectRoot?: string;
  /** Agent ids to inject into. Defaults to claude-code if empty. */
  agents?: string[];
  /** Default git host for owner/repo shorthand. */
  gitHost?: string;
}

export interface McpInstallResult {
  /** Records written to state.mcpServers. */
  records: StoredMcpStandalone[];
  /** Names of agents the servers were injected into. */
  agents: string[];
}

export interface ParsedMcpRef {
  /** The source string after the mcp: prefix (e.g. "user/repo"). */
  inner: string;
  /** Slug used as the namespace pluginName (last path segment of inner). */
  slug: string;
}

// Detect and split an `mcp:<source>` ref. Returns null when the input doesn't
// start with mcp:.
//
// The slug is the last `/`-separated segment of the inner source — used as the
// pluginName component in the `skilltap:<slug>:<server>` namespaced key. For
// `mcp:owner/repo`, slug is `repo`. For `mcp:npm:@scope/name`, slug is `name`.
export function parseMcpRef(source: string): ParsedMcpRef | null {
  if (!source.startsWith("mcp:")) return null;
  const inner = source.slice("mcp:".length);
  if (inner.length === 0) return null;

  const stripped = inner
    .replace(/^https?:\/\//, "")
    .replace(/^git@[^:]+:/, "")
    .replace(/^npm:/, "")
    .replace(/^github:/, "")
    .replace(/\.git$/, "");
  const segments = stripped.split("/").filter(Boolean);
  const last = segments[segments.length - 1] ?? inner;
  // npm scoped packages: drop @ prefix in the slug (e.g. @scope/name → name)
  const slug = last.replace(/^@/, "");
  return { inner, slug };
}

// Install only the MCP servers from a source. Skips skill machinery
// entirely. Servers are namespaced under `skilltap:<slug>:<server-name>`,
// injected into the configured agent configs, and recorded in
// state.json's mcpServers array.
//
// Phase 35b — additive. Doesn't touch v0.x reads/writes.
export async function installMcp(
  source: string,
  options: McpInstallOptions,
): Promise<Result<McpInstallResult, UserError | GitError>> {
  const ref = parseMcpRef(source);
  if (!ref) {
    return err(
      new UserError(
        `Source '${source}' is not an mcp: ref`,
        "Use 'skilltap install mcp:<source>' for MCP-only installs.",
      ),
    );
  }

  const resolved = await resolveSource(ref.inner, options.gitHost);
  if (!resolved.ok) return resolved;

  let contentDir: string;
  let cleanup: (() => Promise<void>) | null = null;

  if (resolved.value.adapter === "local") {
    contentDir = await realpath(resolved.value.url).catch(
      () => resolved.value.url,
    );
  } else {
    const tmpResult = await makeTmpDir();
    if (!tmpResult.ok) return tmpResult;
    const tmp = tmpResult.value;
    cleanup = async () => {
      await removeTmpDir(tmp).catch((e) =>
        debug("mcp-install: cleanup failed", { tmp, error: String(e) }),
      );
    };
    const cloneResult = await clone(resolved.value.url, tmp, {
      branch: resolved.value.ref,
      depth: 1,
    });
    if (!cloneResult.ok) {
      await cleanup();
      return cloneResult;
    }
    contentDir = await realpath(tmp).catch(() => tmp);
  }

  try {
    const servers = await collectServers(contentDir);
    if (!servers.ok) {
      if (cleanup) await cleanup();
      return servers;
    }
    if (servers.value.length === 0) {
      if (cleanup) await cleanup();
      return err(
        new UserError(
          `No MCP servers found in '${ref.inner}'`,
          "Expected a plugin manifest with [[servers]] or a .mcp.json file at the source root.",
        ),
      );
    }

    const agents =
      options.agents && options.agents.length > 0
        ? options.agents
        : ["claude-code"];

    const injectResult = await injectMcpServers({
      pluginName: ref.slug,
      servers: servers.value.map(toStoredMcp),
      agents,
      scope: options.scope,
      projectRoot: options.projectRoot,
    });
    if (!injectResult.ok) {
      if (cleanup) await cleanup();
      return injectResult;
    }

    const stateResult = await loadState(
      options.scope === "project" ? options.projectRoot : undefined,
    );
    if (!stateResult.ok) {
      if (cleanup) await cleanup();
      return stateResult;
    }
    const now = new Date().toISOString();
    const records: StoredMcpStandalone[] = servers.value.map((server) => ({
      name: namespaceMcpServer(ref.slug, server.name),
      source,
      config: serverToStoredConfig(server),
      targets: [...injectResult.value],
      installedAt: now,
    }));
    const newState: State = {
      ...stateResult.value,
      mcpServers: [
        // Replace any existing entry with the same name (idempotent re-install)
        ...stateResult.value.mcpServers.filter(
          (s) => !records.some((r) => r.name === s.name),
        ),
        ...records,
      ],
    };
    const saveResult = await saveState(
      newState,
      options.scope === "project" ? options.projectRoot : undefined,
    );
    if (!saveResult.ok) {
      if (cleanup) await cleanup();
      return saveResult;
    }

    if (cleanup) await cleanup();
    return ok({ records, agents: injectResult.value });
  } catch (e) {
    if (cleanup) await cleanup();
    return err(new UserError(`mcp install failed: ${e}`));
  }
}

export interface McpRemoveOptions {
  scope: "global" | "project";
  projectRoot?: string;
}

export interface McpRemoveResult {
  /** Number of state.mcpServers records removed. */
  removed: number;
  /** Agents that had MCP entries pruned. */
  agents: string[];
  /** Names of removed records (for output). */
  names: string[];
}

// Remove MCP-only installs from a previously-installed source.
// `source` is the same string the user passed to install (e.g., "mcp:user/repo").
// Drops matching entries from state.mcpServers AND removes namespaced keys
// from each target agent's MCP config.
export async function removeMcpInstall(
  source: string,
  options: McpRemoveOptions,
): Promise<Result<McpRemoveResult, UserError>> {
  const stateResult = await loadState(
    options.scope === "project" ? options.projectRoot : undefined,
  );
  if (!stateResult.ok) return stateResult;
  const state = stateResult.value;

  const matching = state.mcpServers.filter((s) => s.source === source);
  if (matching.length === 0) {
    return err(
      new UserError(
        `No MCP servers installed from source '${source}'`,
        "Run 'skilltap status' to see installed MCP servers.",
      ),
    );
  }

  // Group entries by their parsed pluginName so removeMcpServers can prune
  // them per-plugin, per-agent (it removes all keys matching `skilltap:<plugin>:`).
  const byPlugin = new Map<string, Set<string>>();
  for (const entry of matching) {
    const parsed = parseNamespacedKey(entry.name);
    if (!parsed) continue;
    const existing = byPlugin.get(parsed.pluginName) ?? new Set<string>();
    for (const agent of entry.targets) existing.add(agent);
    byPlugin.set(parsed.pluginName, existing);
  }

  const removedAgents = new Set<string>();
  for (const [pluginName, agents] of byPlugin) {
    const result = await removeMcpServers({
      pluginName,
      agents: [...agents],
      scope: options.scope,
      projectRoot: options.projectRoot,
    });
    if (!result.ok) return result;
    for (const a of result.value) removedAgents.add(a);
  }

  const newState: State = {
    ...state,
    mcpServers: state.mcpServers.filter((s) => s.source !== source),
  };
  const saveResult = await saveState(
    newState,
    options.scope === "project" ? options.projectRoot : undefined,
  );
  if (!saveResult.ok) return saveResult;

  return ok({
    removed: matching.length,
    agents: [...removedAgents],
    names: matching.map((s) => s.name),
  });
}

async function collectServers(
  contentDir: string,
): Promise<Result<McpServerEntry[], UserError>> {
  // Prefer plugin manifest's [[servers]] (covers both Claude Code / Codex / .skilltap formats)
  const pluginResult = await detectPlugin(contentDir);
  if (pluginResult.ok && pluginResult.value !== null) {
    const servers = (pluginResult.value as PluginManifest).components
      .filter(
        (c): c is { type: "mcp"; server: McpServerEntry } => c.type === "mcp",
      )
      .map((c) => c.server);
    if (servers.length > 0) return ok(servers);
  }

  // Fall back to a bare .mcp.json at the root
  const mcpResult = await parseMcpJson(join(contentDir, ".mcp.json"));
  if (mcpResult.ok) return ok(mcpResult.value);
  return ok([]);
}

function serverToStoredConfig(
  server: McpServerEntry,
): StoredMcpStandalone["config"] {
  if (server.type === "http") {
    return {
      type: "http",
      url: server.url,
      headers: server.headers ?? {},
    };
  }
  return {
    type: "stdio",
    command: server.command,
    args: server.args ?? [],
    env: server.env ?? {},
  };
}

// Convert an McpServerEntry to a StoredMcpComponent for injectMcpServers.
// (Reuses the shape that injectMcpServers already accepts; this is the same
// transform plugin/state.ts:mcpServerToStored does, inlined to avoid a
// dependency on plugin state types.)
function toStoredMcp(server: McpServerEntry): {
  type: "mcp";
  serverType: "stdio" | "http";
  name: string;
  active: true;
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  url?: string;
  headers?: Record<string, string>;
} {
  if (server.type === "http") {
    return {
      type: "mcp",
      serverType: "http",
      name: server.name,
      active: true,
      url: server.url,
      headers: server.headers ?? {},
    };
  }
  return {
    type: "mcp",
    serverType: "stdio",
    name: server.name,
    active: true,
    command: server.command,
    args: server.args ?? [],
    env: server.env ?? {},
  };
}

/** @deprecated Use `installMcp` instead. Will be removed in a future release. */
export const installMcpOnly = installMcp;
