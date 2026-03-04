import { $ } from "bun";
import { loadInstalled, saveInstalled } from "./config";
import { debug } from "./debug";
import { skillCacheDir, skillInstallDir } from "./paths";
import { wrapShell } from "./shell";
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
      : skillInstallDir(
          record.name,
          record.scope === "linked" ? "global" : record.scope,
          options.projectRoot,
        );
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
