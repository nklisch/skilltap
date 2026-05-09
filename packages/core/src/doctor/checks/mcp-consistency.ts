import { readFile } from "node:fs/promises";
import { fileExists } from "../../fs";
import {
  isNamespacedKey,
  MCP_AGENT_CONFIGS,
  mcpConfigPath,
  parseNamespacedKey,
  removeMcpServers,
} from "../../plugin/mcp-inject";
import { McpClientConfigSchema } from "../../schemas/external/mcp-client-config";
import type { State } from "../../state/schema";
import type { DoctorCheck, DoctorIssue } from "../types";

interface ExpectedEntry {
  pluginName: string;
  serverName: string;
  agent: string;
  scope: "global" | "project";
}

async function readMcpServersFromConfig(
  agent: string,
  scope: "global" | "project",
  projectRoot?: string,
): Promise<Set<string>> {
  const path = mcpConfigPath(agent, scope, projectRoot);
  if (!path) return new Set();
  if (!(await fileExists(path))) return new Set();
  let text: string;
  try {
    text = await readFile(path, "utf8");
  } catch {
    return new Set();
  }
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return new Set();
  }
  const parseResult = McpClientConfigSchema.safeParse(raw);
  if (!parseResult.success || !parseResult.data.mcpServers) return new Set();
  return new Set(Object.keys(parseResult.data.mcpServers));
}

export async function checkMcpConsistency(
  state: State | null,
  projectRoot?: string,
): Promise<DoctorCheck> {
  if (!state) {
    return {
      name: "mcp consistency",
      status: "pass",
      detail: "n/a (no v2 state)",
    };
  }

  const expected: ExpectedEntry[] = [];
  for (const plugin of state.plugins) {
    if (!plugin.active) continue;
    for (const c of plugin.components) {
      if (c.type !== "mcp" || !c.active) continue;
      for (const agent of plugin.also) {
        expected.push({
          pluginName: plugin.name,
          serverName: c.name,
          agent,
          scope: plugin.scope,
        });
      }
    }
  }

  const issues: DoctorIssue[] = [];

  for (const e of expected) {
    const present = await readMcpServersFromConfig(
      e.agent,
      e.scope,
      projectRoot,
    );
    const expectedKey = `skilltap:${e.pluginName}:${e.serverName}`;
    if (!present.has(expectedKey)) {
      issues.push({
        message: `Missing in ${e.agent} (${e.scope}): ${expectedKey}`,
        fixable: false,
      });
    }
  }

  const expectedKeySet = new Set(
    expected.map((e) => `skilltap:${e.pluginName}:${e.serverName}`),
  );

  for (const agent of Object.keys(MCP_AGENT_CONFIGS)) {
    for (const scope of ["global", "project"] as const) {
      if (scope === "project" && !projectRoot) continue;
      const present = await readMcpServersFromConfig(agent, scope, projectRoot);
      for (const key of present) {
        if (!isNamespacedKey(key)) continue;
        if (expectedKeySet.has(key)) continue;
        const parsed = parseNamespacedKey(key);
        if (!parsed) continue;
        issues.push({
          message: `Orphan in ${agent} (${scope}): ${key}`,
          fixable: true,
          fixDescription: `removed orphan from ${agent} config`,
          fix: async () => {
            await removeMcpServers({
              pluginName: parsed.pluginName,
              agents: [agent],
              scope,
              projectRoot,
            });
          },
        });
      }
    }
  }

  if (issues.length === 0) {
    return {
      name: "mcp consistency",
      status: "pass",
      detail:
        expected.length === 0
          ? "n/a (no active MCP servers in state)"
          : `${expected.length} server entries verified`,
    };
  }

  const fixable = issues.filter((i) => i.fixable).length;
  return {
    name: "mcp consistency",
    status: "warn",
    detail: `${issues.length} inconsistenc${issues.length === 1 ? "y" : "ies"} (${fixable} fixable)`,
    issues,
  };
}
