import { mkdir, rename } from "node:fs/promises";
import { dirname } from "node:path";
import { loadSkillState } from "./config";
import { debug } from "./debug";
import { manifestExists, setManifestComponentActive } from "./manifest";
import { skillDisabledDir, skillInstallDir } from "./paths";
import type { InstalledSkill } from "./schemas/installed";
import { applySkillStateChange } from "./state/apply";
import { createAgentSymlinks, removeAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

async function syncDisableToManifest(
  record: InstalledSkill,
  active: boolean,
  projectRoot: string | undefined,
): Promise<void> {
  if (record.scope !== "project") return;
  if (!projectRoot) return;
  if (!record.repo) return;
  if (!(await manifestExists(projectRoot))) return;
  await setManifestComponentActive(
    projectRoot,
    record.repo,
    record.name,
    active,
    "skills",
  ).catch((e) =>
    debug("disable: setManifestComponentActive failed", {
      name: record.name,
      error: String(e),
    }),
  );
}

export type DisableOptions = {
  scope?: "global" | "project" | "linked";
  projectRoot?: string;
};

export type EnableOptions = DisableOptions;

export async function disableSkill(
  name: string,
  options: DisableOptions = {},
): Promise<Result<void, UserError>> {
  const fileRoot =
    options.scope === "project" ? options.projectRoot : undefined;
  const installedResult = await loadSkillState(fileRoot);
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  const record = installed.find(
    (s) => s.name === name && (!options.scope || s.scope === options.scope),
  );

  if (!record) {
    return err(
      new UserError(
        `Skill '${name}' is not installed.`,
        `Run 'skilltap status' to see installed skills.`,
      ),
    );
  }

  if (record.active === false) {
    return err(new UserError(`Skill '${name}' is already disabled.`));
  }

  await removeAgentSymlinks(
    record.name,
    record.also,
    record.scope,
    options.projectRoot,
  );

  if (record.scope !== "linked") {
    const effectiveScope = record.scope === "linked" ? "global" : record.scope;
    const src = skillInstallDir(
      record.name,
      effectiveScope,
      options.projectRoot,
    );
    const dest = skillDisabledDir(
      record.name,
      effectiveScope,
      options.projectRoot,
    );
    await mkdir(dirname(dest), { recursive: true });
    await rename(src, dest);
  }

  const now = new Date().toISOString();
  const applyResult = await applySkillStateChange({
    scope: options.scope === "project" ? "project" : "global",
    projectRoot: options.scope === "project" ? options.projectRoot : undefined,
    mutate: (current) =>
      current.map((s) =>
        s.name === name && (!options.scope || s.scope === options.scope)
          ? { ...s, active: false as const, updatedAt: now }
          : s,
      ),
  });
  if (!applyResult.ok) return applyResult;

  const updated = applyResult.value.find(
    (s) => s.name === name && (!options.scope || s.scope === options.scope),
  ) ?? record;
  await syncDisableToManifest(updated, false, options.projectRoot);

  return ok(undefined);
}

export async function enableSkill(
  name: string,
  options: EnableOptions = {},
): Promise<Result<void, UserError>> {
  const fileRoot =
    options.scope === "project" ? options.projectRoot : undefined;
  const installedResult = await loadSkillState(fileRoot);
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  const record = installed.find(
    (s) => s.name === name && (!options.scope || s.scope === options.scope),
  );

  if (!record) {
    return err(
      new UserError(
        `Skill '${name}' is not installed.`,
        `Run 'skilltap status' to see installed skills.`,
      ),
    );
  }

  if (record.active !== false) {
    return err(new UserError(`Skill '${name}' is already enabled.`));
  }

  let symlinkTarget: string;

  if (record.scope !== "linked") {
    const effectiveScope = record.scope === "linked" ? "global" : record.scope;
    const src = skillDisabledDir(
      record.name,
      effectiveScope,
      options.projectRoot,
    );
    const dest = skillInstallDir(
      record.name,
      effectiveScope,
      options.projectRoot,
    );
    await mkdir(dirname(dest), { recursive: true });
    await rename(src, dest);
    symlinkTarget = dest;
  } else {
    symlinkTarget = record.path ?? "";
  }

  if (record.also.length > 0) {
    const effectiveScope =
      record.scope === "linked"
        ? options.projectRoot
          ? "project"
          : "global"
        : record.scope;
    const symlinkResult = await createAgentSymlinks(
      record.name,
      symlinkTarget,
      record.also,
      effectiveScope,
      options.projectRoot,
    );
    if (!symlinkResult.ok) return symlinkResult;
  }

  const now = new Date().toISOString();
  const applyResult = await applySkillStateChange({
    scope: options.scope === "project" ? "project" : "global",
    projectRoot: options.scope === "project" ? options.projectRoot : undefined,
    mutate: (current) =>
      current.map((s) =>
        s.name === name && (!options.scope || s.scope === options.scope)
          ? { ...s, active: true as const, updatedAt: now }
          : s,
      ),
  });
  if (!applyResult.ok) return applyResult;

  const updated = applyResult.value.find(
    (s) => s.name === name && (!options.scope || s.scope === options.scope),
  ) ?? record;
  await syncDisableToManifest(updated, true, options.projectRoot);

  return ok(undefined);
}
