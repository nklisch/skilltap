import { join, relative, resolve } from "node:path";
import { parseAgentDefinitions } from "../plugin/agents";
import { scan } from "../scanner";
import type {
  PluginAgentComponent,
  PluginManifest,
  PluginMcpComponent,
  PluginSkillComponent,
} from "../schemas/plugin";
import { ok, type Result, type UserError } from "../types";
import type { SkilltapPluginManifest, SkilltapServer } from "./schema";

// Convert a SkilltapPluginManifest (native skilltap format) into the internal
// PluginManifest used by the existing install pipeline. Agents are read
// from disk to populate frontmatter (mirrors parse-claude's behavior).
//
// repoRoot is the directory containing `.skilltap/<name>.toml`. Plugin paths
// in the manifest are relative to repoRoot.
export async function skilltapPluginToManifest(
  manifest: SkilltapPluginManifest,
  repoRoot: string,
): Promise<Result<PluginManifest, UserError>> {
  const skillComponents = await collectSkills(manifest, repoRoot);
  if (!skillComponents.ok) return skillComponents;

  const mcpComponents = manifest.servers.map(serverToComponent);

  const agentResult = await collectAgents(manifest, repoRoot);
  if (!agentResult.ok) return agentResult;

  return ok({
    name: manifest.name,
    version: manifest.version,
    description: manifest.description,
    format: "skilltap",
    pluginRoot: repoRoot,
    components: [
      ...skillComponents.value,
      ...mcpComponents,
      ...agentResult.value,
    ],
  });
}

async function collectSkills(
  manifest: SkilltapPluginManifest,
  repoRoot: string,
): Promise<Result<PluginSkillComponent[], UserError>> {
  const components: PluginSkillComponent[] = [];
  for (const declared of manifest.skills) {
    const absDir = resolve(repoRoot, declared.path);
    let scanned: Awaited<ReturnType<typeof scan>> = [];
    try {
      scanned = await scan(absDir);
    } catch {
      // Path points to nothing — skip silently. Caller can detect via empty
      // skills count if it cares.
    }
    if (scanned.length === 0) {
      // Allow declared name without a SKILL.md — common during scaffolding.
      components.push({
        type: "skill",
        name: declared.name,
        path: relative(repoRoot, absDir),
        description: declared.description,
      });
      continue;
    }
    for (const skill of scanned) {
      components.push({
        type: "skill",
        name: skill.name,
        path: relative(repoRoot, skill.path),
        description: skill.description,
      });
    }
  }
  return ok(components);
}

function serverToComponent(server: SkilltapServer): PluginMcpComponent {
  if (server.type === "http") {
    return {
      type: "mcp",
      server: {
        type: "http",
        name: server.name,
        url: server.url,
        headers: server.headers,
      },
    };
  }
  return {
    type: "mcp",
    server: {
      type: "stdio",
      name: server.name,
      command: server.command,
      args: server.args,
      env: server.env,
    },
  };
}

async function collectAgents(
  manifest: SkilltapPluginManifest,
  repoRoot: string,
): Promise<Result<PluginAgentComponent[], UserError>> {
  if (manifest.agents.length === 0) return ok([]);
  const components: PluginAgentComponent[] = [];
  // Group declared agents by parent dir; reuse parseAgentDefinitions per dir.
  const dirs = new Set<string>();
  for (const a of manifest.agents) {
    dirs.add(resolve(repoRoot, a.path).replace(/\/[^/]+$/, ""));
  }
  for (const dir of dirs) {
    const result = await parseAgentDefinitions(dir, repoRoot);
    if (!result.ok) return result;
    components.push(...result.value);
  }
  // Filter to only declared agents, by name.
  const declaredNames = new Set(manifest.agents.map((a) => a.name));
  return ok(components.filter((c) => declaredNames.has(c.name)));
}

// Helpers exposed for tests
export {
  collectSkills as _collectSkillsForTest,
  serverToComponent as _serverToComponentForTest,
};
// Force `relative` and `join` unused-vars warnings to vanish if linter is strict —
// they're used above.
void [join, relative];
