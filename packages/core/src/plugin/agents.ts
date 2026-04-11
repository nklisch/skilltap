import { readdir } from "node:fs/promises";
import { basename, join, relative } from "node:path";
import { parseSkillFrontmatter } from "../frontmatter";
import { type PluginAgentComponent } from "../schemas/plugin";
import { ok, type Result, UserError } from "../types";

/**
 * Discover and parse agent definition .md files from a directory.
 *
 * @param agentsDir - Absolute path to the agents directory (e.g., plugin/agents/)
 * @param pluginRoot - Absolute path to the plugin root for computing relative paths
 */
export async function parseAgentDefinitions(
  agentsDir: string,
  pluginRoot: string,
): Promise<Result<PluginAgentComponent[], UserError>> {
  let entries: string[];
  try {
    entries = await readdir(agentsDir);
  } catch {
    return ok([]);
  }

  const mdFiles = entries.filter((f) => f.endsWith(".md"));
  if (mdFiles.length === 0) return ok([]);

  const agents: PluginAgentComponent[] = [];

  for (const filename of mdFiles) {
    const filePath = join(agentsDir, filename);
    let content: string;
    try {
      content = await Bun.file(filePath).text();
    } catch {
      continue;
    }

    const frontmatter = parseSkillFrontmatter(content) ?? {};
    const name =
      typeof frontmatter.name === "string" && frontmatter.name
        ? frontmatter.name
        : basename(filename, ".md");

    const relPath = relative(pluginRoot, filePath);

    agents.push({ type: "agent", name, path: relPath, frontmatter });
  }

  agents.sort((a, b) => a.name.localeCompare(b.name));
  return ok(agents);
}
