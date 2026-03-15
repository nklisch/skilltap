import { mkdir, rename } from "node:fs/promises";
import { dirname } from "node:path";
import { loadInstalled, saveInstalled } from "./config";
import { skillDisabledDir, skillInstallDir } from "./paths";
import { createAgentSymlinks, removeAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export type DisableOptions = {
  scope?: "global" | "project" | "linked";
  projectRoot?: string;
};

export type EnableOptions = DisableOptions;

export async function disableSkill(
  name: string,
  options: DisableOptions = {},
): Promise<Result<void, UserError>> {
  const fileRoot = options.scope === "project" ? options.projectRoot : undefined;
  const installedResult = await loadInstalled(fileRoot);
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  const record = installed.skills.find(
    (s) => s.name === name && (!options.scope || s.scope === options.scope),
  );

  if (!record) {
    return err(
      new UserError(
        `Skill '${name}' is not installed.`,
        `Run 'skilltap skills' to see installed skills.`,
      ),
    );
  }

  if (record.active === false) {
    return err(new UserError(`Skill '${name}' is already disabled.`));
  }

  await removeAgentSymlinks(record.name, record.also, record.scope, options.projectRoot);

  if (record.scope !== "linked") {
    const effectiveScope = record.scope === "linked" ? "global" : record.scope;
    const src = skillInstallDir(record.name, effectiveScope, options.projectRoot);
    const dest = skillDisabledDir(record.name, effectiveScope, options.projectRoot);
    await mkdir(dirname(dest), { recursive: true });
    await rename(src, dest);
  }

  record.active = false;
  record.updatedAt = new Date().toISOString();

  const saveResult = await saveInstalled(installed, fileRoot);
  if (!saveResult.ok) return saveResult;

  return ok(undefined);
}

export async function enableSkill(
  name: string,
  options: EnableOptions = {},
): Promise<Result<void, UserError>> {
  const fileRoot = options.scope === "project" ? options.projectRoot : undefined;
  const installedResult = await loadInstalled(fileRoot);
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  const record = installed.skills.find(
    (s) => s.name === name && (!options.scope || s.scope === options.scope),
  );

  if (!record) {
    return err(
      new UserError(
        `Skill '${name}' is not installed.`,
        `Run 'skilltap skills' to see installed skills.`,
      ),
    );
  }

  if (record.active !== false) {
    return err(new UserError(`Skill '${name}' is already enabled.`));
  }

  let symlinkTarget: string;

  if (record.scope !== "linked") {
    const effectiveScope = record.scope === "linked" ? "global" : record.scope;
    const src = skillDisabledDir(record.name, effectiveScope, options.projectRoot);
    const dest = skillInstallDir(record.name, effectiveScope, options.projectRoot);
    await mkdir(dirname(dest), { recursive: true });
    await rename(src, dest);
    symlinkTarget = dest;
  } else {
    symlinkTarget = record.path ?? "";
  }

  if (record.also.length > 0) {
    const effectiveScope =
      record.scope === "linked" ? (options.projectRoot ? "project" : "global") : record.scope;
    const symlinkResult = await createAgentSymlinks(
      record.name,
      symlinkTarget,
      record.also,
      effectiveScope,
      options.projectRoot,
    );
    if (!symlinkResult.ok) return symlinkResult;
  }

  record.active = true;
  record.updatedAt = new Date().toISOString();

  const saveResult = await saveInstalled(installed, fileRoot);
  if (!saveResult.ok) return saveResult;

  return ok(undefined);
}
