import { lstat, readdir, readlink } from "node:fs/promises";
import { join } from "node:path";
import { $ } from "bun";
import { loadInstalled } from "./config";
import { globalBase } from "./fs";
import { parseSkillFrontmatter } from "./frontmatter";
import { findProjectRoot } from "./paths";
import { SkillFrontmatterSchema } from "./schemas/skill";
import type { InstalledSkill } from "./schemas/installed";
import { AGENT_PATHS } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export type SkillLocation = {
  path: string;
  source:
    | { type: "agents"; scope: "global" | "project" }
    | { type: "agent-specific"; agent: string; scope: "global" | "project" };
  isSymlink: boolean;
  symlinkTarget: string | null;
};

export type DiscoveredSkill = {
  name: string;
  managed: boolean;
  record: InstalledSkill | null;
  locations: SkillLocation[];
  gitRemote: string | null;
  description: string;
};

export type DiscoverOptions = {
  global?: boolean;
  project?: boolean;
  projectRoot?: string;
  unmanagedOnly?: boolean;
};

export type DiscoverResult = {
  skills: DiscoveredSkill[];
  managed: number;
  unmanaged: number;
};

async function readDirSafe(dir: string): Promise<string[]> {
  try {
    const entries = await readdir(dir, { withFileTypes: true });
    return entries
      .filter((e) => e.isDirectory() || e.isSymbolicLink())
      .map((e) => e.name);
  } catch {
    return [];
  }
}

async function getSymlinkInfo(
  path: string,
): Promise<{ isSymlink: boolean; target: string | null }> {
  try {
    const stat = await lstat(path);
    if (stat.isSymbolicLink()) {
      const target = await readlink(path);
      return { isSymlink: true, target };
    }
    return { isSymlink: false, target: null };
  } catch {
    return { isSymlink: false, target: null };
  }
}

async function getGitRemote(path: string): Promise<string | null> {
  try {
    const result = await $`git -C ${path} remote get-url origin`.quiet();
    return result.stdout.toString().trim() || null;
  } catch {
    return null;
  }
}

async function getDescription(skillDir: string): Promise<string> {
  try {
    const content = await Bun.file(join(skillDir, "SKILL.md")).text();
    const raw = parseSkillFrontmatter(content);
    if (!raw) return "";
    const result = SkillFrontmatterSchema.safeParse(raw);
    return result.success ? (result.data.description ?? "") : "";
  } catch {
    return "";
  }
}

export async function discoverSkills(
  options?: DiscoverOptions,
): Promise<Result<DiscoverResult, UserError>> {
  const scanGlobal = options?.global !== false && options?.project !== true;
  const scanProject = options?.project !== false && options?.global !== true;

  // Determine project root if needed
  let projectRoot: string | undefined;
  if (scanProject) {
    projectRoot = options?.projectRoot ?? (await findProjectRoot());
  }

  // Load installed records for cross-referencing
  const globalInstalledResult = scanGlobal ? await loadInstalled() : null;
  if (globalInstalledResult && !globalInstalledResult.ok) {
    return globalInstalledResult;
  }
  const globalInstalled = globalInstalledResult?.value.skills ?? [];

  const projectInstalledResult =
    scanProject && projectRoot ? await loadInstalled(projectRoot) : null;
  if (projectInstalledResult && !projectInstalledResult.ok) {
    return projectInstalledResult;
  }
  const projectInstalled = projectInstalledResult?.value.skills ?? [];

  // Build a map of name -> InstalledSkill from all scopes
  const installedMap = new Map<string, InstalledSkill>();
  for (const skill of globalInstalled) {
    installedMap.set(skill.name, skill);
  }
  for (const skill of projectInstalled) {
    installedMap.set(skill.name, skill);
  }

  // Map from canonical real path -> DiscoveredSkill (for deduplication)
  const byPath = new Map<string, DiscoveredSkill>();
  // Map from name -> DiscoveredSkill (for deduplication when path not resolved)
  const byName = new Map<string, DiscoveredSkill>();

  async function processEntry(
    name: string,
    entryPath: string,
    source: SkillLocation["source"],
  ): Promise<void> {
    const symlinkInfo = await getSymlinkInfo(entryPath);

    // If this is a symlink, try to resolve to the real path for deduplication
    if (symlinkInfo.isSymlink && symlinkInfo.target) {
      // Resolve the target to find if we've already seen it
      const resolvedTarget = symlinkInfo.target.startsWith("/")
        ? symlinkInfo.target
        : join(entryPath, "..", symlinkInfo.target);

      const existing = byPath.get(resolvedTarget) ?? byName.get(name);
      if (existing) {
        // Add this as an additional location
        existing.locations.push({
          path: entryPath,
          source,
          isSymlink: true,
          symlinkTarget: symlinkInfo.target,
        });
        return;
      }
    }

    // Check if already seen by name
    const existingByName = byName.get(name);
    if (existingByName) {
      existingByName.locations.push({
        path: entryPath,
        source,
        isSymlink: symlinkInfo.isSymlink,
        symlinkTarget: symlinkInfo.target,
      });
      return;
    }

    const record = installedMap.get(name) ?? null;
    const managed = record !== null;

    // For real directories (not symlinks), get description and git remote
    let description = "";
    let gitRemote: string | null = null;
    if (!symlinkInfo.isSymlink) {
      description = await getDescription(entryPath);
      if (!managed) {
        gitRemote = await getGitRemote(entryPath);
      }
    }

    const skill: DiscoveredSkill = {
      name,
      managed,
      record,
      locations: [
        {
          path: entryPath,
          source,
          isSymlink: symlinkInfo.isSymlink,
          symlinkTarget: symlinkInfo.target,
        },
      ],
      gitRemote,
      description,
    };

    byPath.set(entryPath, skill);
    byName.set(name, skill);
  }

  // Scan global scope
  if (scanGlobal) {
    const base = globalBase();

    // Scan .agents/skills/
    const agentsDir = join(base, ".agents", "skills");
    const agentsEntries = await readDirSafe(agentsDir);
    for (const name of agentsEntries) {
      if (name === ".disabled") continue;
      await processEntry(name, join(agentsDir, name), {
        type: "agents",
        scope: "global",
      });
    }

    // Scan .agents/skills/.disabled/ for disabled skills
    const disabledDir = join(agentsDir, ".disabled");
    const disabledEntries = await readDirSafe(disabledDir);
    for (const name of disabledEntries) {
      await processEntry(name, join(disabledDir, name), {
        type: "agents",
        scope: "global",
      });
    }

    // Scan each agent-specific dir
    for (const [agent, relDir] of Object.entries(AGENT_PATHS)) {
      const agentDir = join(base, relDir);
      const agentEntries = await readDirSafe(agentDir);
      for (const name of agentEntries) {
        await processEntry(name, join(agentDir, name), {
          type: "agent-specific",
          agent,
          scope: "global",
        });
      }
    }
  }

  // Scan project scope
  if (scanProject && projectRoot) {
    const base = projectRoot;

    // Scan .agents/skills/
    const agentsDir = join(base, ".agents", "skills");
    const agentsEntries = await readDirSafe(agentsDir);
    for (const name of agentsEntries) {
      if (name === ".disabled") continue;
      await processEntry(name, join(agentsDir, name), {
        type: "agents",
        scope: "project",
      });
    }

    // Scan .agents/skills/.disabled/ for disabled skills
    const disabledDir = join(agentsDir, ".disabled");
    const disabledEntries = await readDirSafe(disabledDir);
    for (const name of disabledEntries) {
      await processEntry(name, join(disabledDir, name), {
        type: "agents",
        scope: "project",
      });
    }

    // Scan each agent-specific dir
    for (const [agent, relDir] of Object.entries(AGENT_PATHS)) {
      const agentDir = join(base, relDir);
      const agentEntries = await readDirSafe(agentDir);
      for (const name of agentEntries) {
        await processEntry(name, join(agentDir, name), {
          type: "agent-specific",
          agent,
          scope: "project",
        });
      }
    }
  }

  let skills = Array.from(byName.values());

  if (options?.unmanagedOnly) {
    skills = skills.filter((s) => !s.managed);
  }

  const managed = skills.filter((s) => s.managed).length;
  const unmanaged = skills.filter((s) => !s.managed).length;

  return ok({ skills, managed, unmanaged });
}
