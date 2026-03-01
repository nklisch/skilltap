import { mkdir, symlink } from "node:fs/promises";
import { dirname, join } from "node:path";
import { loadInstalled, saveInstalled } from "./config";
import { skillInstallDir } from "./paths";
import { scan } from "./scanner";
import type { InstalledSkill } from "./schemas/installed";
import { createAgentSymlinks } from "./symlink";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

export type LinkOptions = {
  scope: "global" | "project";
  projectRoot?: string;
  also?: string[];
};

export async function linkSkill(
  localPath: string,
  options: LinkOptions,
): Promise<Result<InstalledSkill, UserError>> {
  // 1. Validate localPath has SKILL.md
  const skillMdFile = Bun.file(join(localPath, "SKILL.md"));
  if (!(await skillMdFile.exists())) {
    return err(
      new UserError(
        `"${localPath}" does not contain SKILL.md`,
        "The path must be a valid skill directory.",
      ),
    );
  }

  // 2. Get skill name via scan
  const scanned = await scan(localPath);
  if (scanned.length === 0) {
    return err(new UserError(`No skill found in "${localPath}"`));
  }
  // biome-ignore lint/style/noNonNullAssertion: scanned.length > 0
  const skill = scanned[0]!;

  // 3. Load installed to check for conflicts
  const installedResult = await loadInstalled();
  if (!installedResult.ok) return installedResult;
  const installed = installedResult.value;

  // 4. Check already-installed
  const conflict = installed.skills.find((s) => s.name === skill.name);
  if (conflict) {
    return err(
      new UserError(
        `Skill '${skill.name}' is already installed.`,
        `Run 'skilltap remove ${skill.name}' first.`,
      ),
    );
  }

  // 5. Compute install path and create symlink
  const installPath = skillInstallDir(
    skill.name,
    options.scope,
    options.projectRoot,
  );
  await mkdir(dirname(installPath), { recursive: true });

  try {
    await symlink(localPath, installPath, "dir");
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return err(new UserError(`Failed to create symlink: ${msg}`));
  }

  // 6. Create agent symlinks if requested
  const also = options.also ?? [];
  if (also.length > 0) {
    const symlinkResult = await createAgentSymlinks(
      skill.name,
      installPath,
      also,
      options.scope,
      options.projectRoot,
    );
    if (!symlinkResult.ok) return symlinkResult;
  }

  // 7. Build record (path = installPath = the symlink location)
  const now = new Date().toISOString();
  const record: InstalledSkill = {
    name: skill.name,
    description: skill.description,
    repo: null,
    ref: null,
    sha: null,
    scope: "linked",
    path: installPath,
    tap: null,
    also,
    installedAt: now,
    updatedAt: now,
  };

  // 8. Save installed.json
  installed.skills.push(record);
  const saveResult = await saveInstalled(installed);
  if (!saveResult.ok) return saveResult;

  return ok(record);
}
