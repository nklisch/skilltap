import { unlink } from "node:fs/promises";
import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import { debug } from "./debug";
import type { DiscoveredSkill, SkillLocation } from "./discover";
import { resolvedDirExists } from "./fs";
import { skillCacheDir, skillDisabledDir, skillInstallDir } from "./paths";
import { wrapShell } from "./shell";
import { removeAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export type RemoveOptions = {
  scope?: "global" | "project" | "linked";
  projectRoot?: string;
  /** Called when removing a skill whose directory was already missing. */
  onOrphanRemoved?: (name: string) => void;
};

export async function removeSkill(
  name: string,
  options: RemoveOptions = {},
): Promise<Result<void, UserError>> {
  debug("removeSkill", { name, scope: options.scope });
  const fileRoot = options.scope === "project" ? options.projectRoot : undefined;
  const installedResult = await loadInstalled(fileRoot);
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  const idx = installed.skills.findIndex(
    (s) => s.name === name && (!options.scope || s.scope === options.scope),
  );

  if (idx === -1) {
    return err(
      new UserError(
        `Skill '${name}' is not installed.`,
        `Run 'skilltap list' to see installed skills.`,
      ),
    );
  }

  // biome-ignore lint/style/noNonNullAssertion: idx was found via findIndex, guaranteed in range
  const record = installed.skills[idx]!;

  // Remove agent symlinks
  await removeAgentSymlinks(
    record.name,
    record.also,
    record.scope,
    options.projectRoot,
  );

  // Remove skill directory (for linked skills, record.path is the symlink location)
  const installPath =
    record.scope === "linked" && record.path !== null
      ? record.path
      : record.active === false
        ? skillDisabledDir(
            record.name,
            record.scope === "linked" ? "global" : record.scope,
            options.projectRoot,
          )
        : skillInstallDir(
            record.name,
            record.scope === "linked" ? "global" : record.scope,
            options.projectRoot,
          );
  if (!(await resolvedDirExists(installPath))) {
    options.onOrphanRemoved?.(name);
  }

  const rmResult = await wrapShell(
    () => $`rm -rf ${installPath}`.quiet().then(() => undefined),
    `Failed to remove skill directory '${name}'`,
    "Check file permissions.",
  );
  if (!rmResult.ok) return rmResult;

  // Remove cache if this was the last skill from the repo
  if (record.path !== null && record.repo) {
    const remainingFromSameRepo = installed.skills.filter(
      (s, i) => i !== idx && s.repo === record.repo,
    );
    if (remainingFromSameRepo.length === 0) {
      const cacheRoot = skillCacheDir(record.repo);
      const cacheResult = await wrapShell(
        () => $`rm -rf ${cacheRoot}`.quiet().then(() => undefined),
        `Failed to remove cache directory for '${name}'`,
      );
      if (!cacheResult.ok) {
        debug("cache cleanup failed", { name, cacheRoot, error: cacheResult.error.message });
      }
    }
  }

  installed.skills.splice(idx, 1);
  const saveResult = await saveInstalled(installed, fileRoot);
  if (!saveResult.ok) return saveResult;

  return ok(undefined);
}

export type RemoveAnyOptions = {
  skill: DiscoveredSkill;
  removeAll?: boolean;
  locations?: SkillLocation[];
};

export async function removeAnySkill(
  options: RemoveAnyOptions,
): Promise<Result<void, UserError>> {
  const { skill } = options;

  if (skill.managed && skill.record) {
    return removeSkill(skill.name, { scope: skill.record.scope });
  }

  // Determine which locations to remove
  let locs: SkillLocation[];
  if (options.locations) {
    locs = options.locations;
  } else if (options.removeAll) {
    locs = skill.locations;
  } else {
    // Default: remove only the first non-symlink location
    const primary = skill.locations.find((l) => !l.isSymlink);
    locs = primary ? [primary] : skill.locations.slice(0, 1);
  }

  for (const loc of locs) {
    if (loc.isSymlink) {
      try {
        await unlink(loc.path);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        return err(new UserError(`Failed to remove symlink '${loc.path}': ${msg}`));
      }
    } else {
      const rmResult = await wrapShell(
        () => $`rm -rf ${loc.path}`.quiet().then(() => undefined),
        `Failed to remove skill directory '${loc.path}'`,
        "Check file permissions.",
      );
      if (!rmResult.ok) return rmResult;
    }
  }

  return ok(undefined);
}
