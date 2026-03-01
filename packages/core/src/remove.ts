import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import { skillCacheDir, skillInstallDir } from "./paths";
import { removeAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export type RemoveOptions = {
  scope?: "global" | "project" | "linked";
  projectRoot?: string;
};

export async function removeSkill(
  name: string,
  options: RemoveOptions = {},
): Promise<Result<void, UserError>> {
  const installedResult = await loadInstalled();
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
      : skillInstallDir(
          record.name,
          record.scope === "linked" ? "global" : record.scope,
          options.projectRoot,
        );
  await $`rm -rf ${installPath}`.quiet();

  // Remove cache if this was the last skill from the repo
  if (record.path !== null && record.repo) {
    const remainingFromSameRepo = installed.skills.filter(
      (s, i) => i !== idx && s.repo === record.repo,
    );
    if (remainingFromSameRepo.length === 0) {
      const cacheRoot = skillCacheDir(record.repo);
      await $`rm -rf ${cacheRoot}`.quiet();
    }
  }

  installed.skills.splice(idx, 1);
  const saveResult = await saveInstalled(installed);
  if (!saveResult.ok) return saveResult;

  return ok(undefined);
}
