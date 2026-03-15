import { mkdir, symlink } from "node:fs/promises";
import { dirname } from "node:path";
import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import type { DiscoveredSkill } from "./discover";
import { globalBase } from "./fs";
import { revParse } from "./git";
import { skillInstallDir } from "./paths";
import type { InstalledSkill } from "./schemas/installed";
import type { StaticWarning } from "./security/static";
import { scanStatic } from "./security/static";
import { AGENT_PATHS, createAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export type AdoptMode = "move" | "track-in-place";

export type AdoptOptions = {
  mode?: AdoptMode;
  scope?: "global" | "project";
  projectRoot?: string;
  also?: string[];
  skipScan?: boolean;
  onWarnings?: (warnings: StaticWarning[], skillName: string) => Promise<boolean>;
};

export type AdoptResult = {
  record: InstalledSkill;
  symlinksCreated: string[];
};

export async function adoptSkill(
  skill: DiscoveredSkill,
  options?: AdoptOptions,
): Promise<Result<AdoptResult, UserError>> {
  if (skill.managed) {
    return err(
      new UserError(
        `Skill '${skill.name}' is already managed by skilltap.`,
        `Run 'skilltap list' to see managed skills.`,
      ),
    );
  }

  const mode = options?.mode ?? "move";
  const scope = options?.scope ?? "global";
  const projectRoot = options?.projectRoot;
  const also = options?.also ?? [];

  // Find the primary location (first non-symlink, or first overall)
  const primaryLocation =
    skill.locations.find((l) => !l.isSymlink) ?? skill.locations[0];

  if (!primaryLocation) {
    return err(new UserError(`Skill '${skill.name}' has no locations to adopt.`));
  }

  const srcPath = primaryLocation.path;

  // Security scan
  if (!options?.skipScan) {
    const scanResult = await scanStatic(srcPath);
    if (!scanResult.ok) return scanResult;
    const warnings = scanResult.value;
    if (warnings.length > 0 && options?.onWarnings) {
      const proceed = await options.onWarnings(warnings, skill.name);
      if (!proceed) {
        return err(new UserError(`Adopt of '${skill.name}' aborted due to security warnings.`));
      }
    }
  }

  // Get git info if available
  let gitRemote = skill.gitRemote ?? null;
  let branch: string | null = null;
  let sha: string | null = null;

  {
    // Try to get branch and SHA from the source path (best-effort; many skills aren't git repos)
    try {
      const branchResult =
        await $`git -C ${srcPath} rev-parse --abbrev-ref HEAD`.quiet();
      branch = branchResult.stdout.toString().trim() || null;
    } catch {
      branch = null;
    }
    const shaResult = await revParse(srcPath);
    if (shaResult.ok) {
      sha = shaResult.value;
    }
  }

  const now = new Date().toISOString();
  const symlinksCreated: string[] = [];

  if (mode === "move") {
    const targetPath = skillInstallDir(skill.name, scope, projectRoot);

    let actualPath = srcPath;

    if (srcPath !== targetPath) {
      // Ensure parent dir exists
      try {
        await mkdir(dirname(targetPath), { recursive: true });
      } catch (e) {
        return err(new UserError(`Failed to create target directory: ${e}`));
      }

      // Move the skill directory
      try {
        await $`mv ${srcPath} ${targetPath}`.quiet();
        actualPath = targetPath;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        return err(new UserError(`Failed to move skill directory: ${msg}`));
      }

      // Create symlinks back from original real-directory locations
      for (const loc of skill.locations) {
        if (!loc.isSymlink && loc.path !== srcPath) {
          try {
            await mkdir(dirname(loc.path), { recursive: true });
            await symlink(targetPath, loc.path, "dir");
            symlinksCreated.push(loc.path);
          } catch {
            // Best effort — ignore if already exists or similar
          }
        }
      }

      // Create symlink back at original location if it was a real directory
      if (!primaryLocation.isSymlink) {
        try {
          await mkdir(dirname(srcPath), { recursive: true });
          await symlink(targetPath, srcPath, "dir");
          symlinksCreated.push(srcPath);
        } catch {
          // Best effort
        }
      }
    }

    // Create agent symlinks
    const symlinkResult = await createAgentSymlinks(
      skill.name,
      actualPath,
      also,
      scope,
      projectRoot,
    );
    if (!symlinkResult.ok) return symlinkResult;
    for (const agent of also) {
      const relDir = AGENT_PATHS[agent];
      if (relDir) {
        const base =
          scope === "global" ? globalBase() : (projectRoot ?? process.cwd());
        const agentLinkPath = `${base}/${relDir}/${skill.name}`;
        symlinksCreated.push(agentLinkPath);
      }
    }

    // Build and save record
    const record: InstalledSkill = {
      name: skill.name,
      description: skill.description,
      repo: gitRemote,
      ref: branch,
      sha,
      scope,
      path: null,
      tap: null,
      also,
      installedAt: now,
      updatedAt: now,
    };

    const fileRoot = scope === "project" ? projectRoot : undefined;
    const installedResult = await loadInstalled(fileRoot);
    if (!installedResult.ok) return installedResult;
    const installed = installedResult.value;
    installed.skills.push(record);
    const saveResult = await saveInstalled(installed, fileRoot);
    if (!saveResult.ok) return saveResult;

    return ok({ record, symlinksCreated });
  } else {
    // track-in-place mode
    const record: InstalledSkill = {
      name: skill.name,
      description: skill.description,
      repo: gitRemote,
      ref: branch,
      sha,
      scope: "linked",
      path: srcPath,
      tap: null,
      also,
      installedAt: now,
      updatedAt: now,
    };

    // Create agent symlinks pointing to the current location
    const symlinkResult = await createAgentSymlinks(
      skill.name,
      srcPath,
      also,
      scope,
      projectRoot,
    );
    if (!symlinkResult.ok) return symlinkResult;

    // Save record to appropriate installed.json
    const fileRoot = scope === "project" ? projectRoot : undefined;
    const installedResult = await loadInstalled(fileRoot);
    if (!installedResult.ok) return installedResult;
    const installed = installedResult.value;
    installed.skills.push(record);
    const saveResult = await saveInstalled(installed, fileRoot);
    if (!saveResult.ok) return saveResult;

    return ok({ record, symlinksCreated });
  }
}
