import { mkdir, rename, symlink } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import { $ } from "bun";
import type { DiscoveredAgentPlugin } from "./agent-plugins/types";
import { scanAllAgentPlugins } from "./agent-plugins/registry";
import { loadSkillState, saveSkillState } from "./config";
import { debug } from "./debug";
import type { DiscoverOptions, DiscoveredSkill } from "./discover";
import { discoverSkills } from "./discover";
import { revParse } from "./git";
import {
  addPluginToManifest,
  addSkillToManifest,
  manifestExists,
} from "./manifest";
import { scopeBase, skillInstallDir } from "./paths";
import { addPlugin, loadPlugins, manifestToRecord, savePlugins } from "./plugin/state";
import type { PluginRecord } from "./schemas/plugins";
import type { InstalledSkill } from "./schemas/installed";
import { scan } from "./scanner";
import type { StaticWarning } from "./security/static";
import { scanStatic } from "./security/static";
import { AGENT_PATHS, createAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";
import type { ScanAllResult } from "./agent-plugins/registry";

// Best-effort manifest+lockfile sync for adopted records. Mirrors the install
// path: only writes when a project manifest already exists, and swallows
// errors via `debug()` so a manifest hiccup never breaks the adopt flow.
async function syncAdoptToManifest(
  record: InstalledSkill,
  projectRoot: string | undefined,
): Promise<void> {
  if (record.scope !== "project") return;
  if (!projectRoot) return;
  if (!record.repo) return;
  if (!(await manifestExists(projectRoot))) return;
  await addSkillToManifest(projectRoot, {
    source: record.repo,
    ref: record.ref,
    sha: record.sha,
  }).catch((e) =>
    debug("adopt: addSkillToManifest failed", {
      name: record.name,
      error: String(e),
    }),
  );
}

async function syncAdoptPluginToManifest(
  record: PluginRecord,
  projectRoot: string | undefined,
): Promise<void> {
  if (record.scope !== "project") return;
  if (!projectRoot) return;
  if (!record.repo) return;
  if (!(await manifestExists(projectRoot))) return;
  await addPluginToManifest(projectRoot, {
    source: record.repo,
    ref: record.ref ?? null,
    sha: record.sha ?? null,
  }).catch((e) =>
    debug("adopt: addPluginToManifest failed", {
      name: record.name,
      error: String(e),
    }),
  );
}

export type AdoptMode = "move" | "track-in-place";

export type AdoptOptions = {
  mode?: AdoptMode;
  scope?: "global" | "project";
  projectRoot?: string;
  also?: string[];
  skipScan?: boolean;
  onWarnings?: (
    warnings: StaticWarning[],
    skillName: string,
  ) => Promise<boolean>;
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
    return err(
      new UserError(`Skill '${skill.name}' has no locations to adopt.`),
    );
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
        return err(
          new UserError(
            `Adopt of '${skill.name}' aborted due to security warnings.`,
          ),
        );
      }
    }
  }

  // Get git info if available
  const gitRemote = skill.gitRemote ?? null;
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
        const agentLinkPath = `${scopeBase(scope, projectRoot)}/${relDir}/${skill.name}`;
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
    const installedResult = await loadSkillState(fileRoot);
    if (!installedResult.ok) return installedResult;
    const installed = installedResult.value;
    installed.skills.push(record);
    const saveResult = await saveSkillState(installed, fileRoot);
    if (!saveResult.ok) return saveResult;
    await syncAdoptToManifest(record, projectRoot);

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

    // Save record to appropriate state
    const fileRoot = scope === "project" ? projectRoot : undefined;
    const installedResult = await loadSkillState(fileRoot);
    if (!installedResult.ok) return installedResult;
    const installed = installedResult.value;
    installed.skills.push(record);
    const saveResult = await saveSkillState(installed, fileRoot);
    if (!saveResult.ok) return saveResult;
    await syncAdoptToManifest(record, projectRoot);

    return ok({ record, symlinksCreated });
  }
}

export type AdoptFromPathOptions = {
  scope?: "global" | "project";
  projectRoot?: string;
  also?: string[];
  /** "track-in-place" (default; symlink) or "move" (relocate dir + symlink back). */
  mode?: AdoptMode;
  skipScan?: boolean;
  onWarnings?: (warnings: StaticWarning[], skillName: string) => Promise<boolean>;
};

export type DiscoverAdoptableResult = {
  skills: DiscoveredSkill[];
  plugins: DiscoveredAgentPlugin[];
  scannerErrors: ScanAllResult["errors"];
};

/**
 * Adopt a skill from an arbitrary on-disk path. Replaces the deleted `link`
 * command. Default mode: track-in-place — symlinks the path into the
 * canonical agent dir. With mode: "move", relocates the dir.
 *
 * Validates that the path contains a SKILL.md before doing anything.
 */
export async function adoptSkillFromPath(
  path: string,
  options: AdoptFromPathOptions,
): Promise<Result<AdoptResult, UserError>> {
  const skillMdFile = Bun.file(join(path, "SKILL.md"));
  if (!(await skillMdFile.exists())) {
    return err(
      new UserError(
        `"${path}" does not contain SKILL.md`,
        "The path must be a valid skill directory with a SKILL.md file.",
      ),
    );
  }

  const scanned = await scan(path);
  if (scanned.length === 0) {
    return err(new UserError(`No skill found in "${path}"`));
  }
  // biome-ignore lint/style/noNonNullAssertion: scanned.length > 0
  const scannedSkill = scanned[0]!;

  const mode = options?.mode ?? "track-in-place";
  const scope = options?.scope ?? "global";
  const projectRoot = options?.projectRoot;
  const also = options?.also ?? [];

  // Security scan
  if (!options?.skipScan) {
    const scanResult = await scanStatic(path);
    if (!scanResult.ok) return scanResult;
    const warnings = scanResult.value;
    if (warnings.length > 0 && options?.onWarnings) {
      const proceed = await options.onWarnings(warnings, scannedSkill.name);
      if (!proceed) {
        return err(
          new UserError(
            `Adopt of '${scannedSkill.name}' aborted due to security warnings.`,
          ),
        );
      }
    }
  }

  const now = new Date().toISOString();
  const symlinksCreated: string[] = [];

  if (mode === "move") {
    const targetPath = skillInstallDir(scannedSkill.name, scope, projectRoot);

    if (path !== targetPath) {
      try {
        await mkdir(dirname(targetPath), { recursive: true });
        await rename(path, targetPath);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        return err(new UserError(`Failed to move skill directory: ${msg}`));
      }

      // Create symlink back at original location
      try {
        await mkdir(dirname(path), { recursive: true });
        await symlink(targetPath, path, "dir");
        symlinksCreated.push(path);
      } catch {
        // Best effort
      }
    }

    const symlinkResult = await createAgentSymlinks(
      scannedSkill.name,
      targetPath,
      also,
      scope,
      projectRoot,
    );
    if (!symlinkResult.ok) return symlinkResult;
    for (const agent of also) {
      const relDir = AGENT_PATHS[agent];
      if (relDir) {
        const agentLinkPath = `${scopeBase(scope, projectRoot)}/${relDir}/${scannedSkill.name}`;
        symlinksCreated.push(agentLinkPath);
      }
    }

    const record: InstalledSkill = {
      name: scannedSkill.name,
      description: scannedSkill.description,
      repo: null,
      ref: null,
      sha: null,
      scope,
      path: null,
      tap: null,
      also,
      installedAt: now,
      updatedAt: now,
    };

    const fileRoot = scope === "project" ? projectRoot : undefined;
    const installedResult = await loadSkillState(fileRoot);
    if (!installedResult.ok) return installedResult;
    const installed = installedResult.value;
    installed.skills.push(record);
    const saveResult = await saveSkillState(installed, fileRoot);
    if (!saveResult.ok) return saveResult;
    await syncAdoptToManifest(record, projectRoot);

    return ok({ record, symlinksCreated });
  } else {
    // track-in-place: symlink into canonical agent dir
    const installPath = skillInstallDir(scannedSkill.name, scope, projectRoot);
    try {
      await mkdir(dirname(installPath), { recursive: true });
      await symlink(path, installPath, "dir");
      symlinksCreated.push(installPath);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      return err(new UserError(`Failed to create symlink: ${msg}`));
    }

    const symlinkResult = await createAgentSymlinks(
      scannedSkill.name,
      installPath,
      also,
      scope,
      projectRoot,
    );
    if (!symlinkResult.ok) return symlinkResult;
    for (const agent of also) {
      const relDir = AGENT_PATHS[agent];
      if (relDir) {
        const agentLinkPath = `${scopeBase(scope, projectRoot)}/${relDir}/${scannedSkill.name}`;
        symlinksCreated.push(agentLinkPath);
      }
    }

    const record: InstalledSkill = {
      name: scannedSkill.name,
      description: scannedSkill.description,
      repo: null,
      ref: null,
      sha: null,
      scope: "linked",
      path,
      tap: null,
      also,
      installedAt: now,
      updatedAt: now,
    };

    const fileRoot = scope === "project" ? projectRoot : undefined;
    const installedResult = await loadSkillState(fileRoot);
    if (!installedResult.ok) return installedResult;
    const installed = installedResult.value;
    installed.skills.push(record);
    const saveResult = await saveSkillState(installed, fileRoot);
    if (!saveResult.ok) return saveResult;
    await syncAdoptToManifest(record, projectRoot);

    return ok({ record, symlinksCreated });
  }
}

/**
 * Adopt a Claude-Code-managed plugin into skilltap state. Doesn't copy or
 * move files; just adds a state.plugins[] entry that points at the
 * installPath. The plugin remains owned by Claude Code; removing from
 * skilltap doesn't uninstall from Claude Code (out of scope).
 */
export async function adoptPlugin(
  plugin: DiscoveredAgentPlugin,
  options: { also?: string[]; projectRoot?: string },
): Promise<Result<{ record: PluginRecord }, UserError>> {
  const also = options.also ?? [];

  // Build a recognizable repo marker so doctor/status can identify adopted plugins
  let repo: string;
  if (plugin.sourceUrl) {
    repo = plugin.sourceUrl;
  } else if (plugin.marketplaceName) {
    repo = `claude-code:${plugin.marketplaceName}:${plugin.name}`;
  } else {
    repo = `claude-code:${plugin.name}`;
  }

  const record = manifestToRecord(plugin.manifest, {
    repo,
    ref: null,
    sha: plugin.sha,
    scope: plugin.scope,
    also,
    tap: null,
  });

  // Override path to point at Claude Code's cache
  const recordWithPath: PluginRecord = { ...record, path: plugin.installPath };

  const projectRoot = plugin.scope === "project" ? (options.projectRoot ?? plugin.projectRoot) : undefined;
  const pluginsResult = await loadPlugins(projectRoot);
  if (!pluginsResult.ok) return pluginsResult;
  const updated = addPlugin(pluginsResult.value, recordWithPath);
  const saveResult = await savePlugins(updated, projectRoot);
  if (!saveResult.ok) return saveResult;
  await syncAdoptPluginToManifest(recordWithPath, projectRoot);

  return ok({ record: recordWithPath });
}

/**
 * Unified discovery: scan unmanaged skills (existing discoverSkills) +
 * scan all registered AgentPluginScanners. Returns a combined result the
 * CLI picker can show.
 */
export async function discoverAllAdoptable(
  options: DiscoverOptions,
): Promise<Result<DiscoverAdoptableResult, UserError>> {
  const skillsResult = await discoverSkills({ ...options, unmanagedOnly: true });
  if (!skillsResult.ok) return skillsResult;

  const pluginScanResult = await scanAllAgentPlugins();
  if (!pluginScanResult.ok) {
    return ok({
      skills: skillsResult.value.skills,
      plugins: [],
      scannerErrors: [{ scanner: "registry", error: pluginScanResult.error }],
    });
  }

  return ok({
    skills: skillsResult.value.skills,
    plugins: pluginScanResult.value.plugins,
    scannerErrors: pluginScanResult.value.errors,
  });
}
