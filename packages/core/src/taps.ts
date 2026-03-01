import { rm } from "node:fs/promises";
import { join } from "node:path";
import { $ } from "bun";
import { z } from "zod/v4";
import { getConfigDir, loadConfig, saveConfig } from "./config";
import { checkGitInstalled, clone, pull } from "./git";
import type { Tap, TapSkill } from "./schemas/tap";
import { TapSchema } from "./schemas/tap";
import { err, type GitError, ok, type Result, UserError } from "./types";

export type TapEntry = { tapName: string; skill: TapSkill };

function tapDir(name: string): string {
  return join(getConfigDir(), "taps", name);
}

async function loadTapJson(
  dir: string,
  name?: string,
): Promise<Result<Tap, UserError>> {
  const label = name ? `tap '${name}'` : dir;
  const file = Bun.file(join(dir, "tap.json"));
  if (!(await file.exists())) {
    return err(new UserError(`tap.json not found in ${label}`));
  }
  let raw: unknown;
  try {
    raw = await file.json();
  } catch (e) {
    return err(new UserError(`Invalid JSON in tap.json in ${label}: ${e}`));
  }
  const result = TapSchema.safeParse(raw);
  if (!result.success) {
    const details = z.prettifyError(result.error);
    return err(
      new UserError(`Invalid tap.json in ${label}: ${details}`),
    );
  }
  return ok(result.data);
}

export async function addTap(
  name: string,
  url: string,
): Promise<Result<{ skillCount: number }, UserError | GitError>> {
  const configResult = await loadConfig();
  if (!configResult.ok) return configResult;
  const config = configResult.value;

  if (config.taps.some((t) => t.name === name)) {
    return err(
      new UserError(
        `Tap '${name}' already exists.`,
        `Remove it first with 'skilltap tap remove ${name}'`,
      ),
    );
  }

  const gitCheck = await checkGitInstalled();
  if (!gitCheck.ok) return gitCheck;

  const dest = tapDir(name);
  const cloneResult = await clone(url, dest, { depth: 1 });
  if (!cloneResult.ok) return cloneResult;

  const tapResult = await loadTapJson(dest, name);
  if (!tapResult.ok) {
    await rm(dest, { recursive: true, force: true });
    return tapResult;
  }

  config.taps.push({ name, url });
  const saveResult = await saveConfig(config);
  if (!saveResult.ok) return saveResult;

  return ok({ skillCount: tapResult.value.skills.length });
}

export async function removeTap(
  name: string,
): Promise<Result<void, UserError>> {
  const configResult = await loadConfig();
  if (!configResult.ok) return configResult;
  const config = configResult.value;

  const idx = config.taps.findIndex((t) => t.name === name);
  if (idx === -1) {
    return err(
      new UserError(
        `Tap '${name}' is not configured.`,
        `Run 'skilltap tap list' to see configured taps.`,
      ),
    );
  }

  config.taps.splice(idx, 1);
  const saveResult = await saveConfig(config);
  if (!saveResult.ok) return saveResult;

  await rm(tapDir(name), { recursive: true, force: true });
  return ok(undefined);
}

export async function updateTap(
  name?: string,
): Promise<Result<Record<string, number>, UserError | GitError>> {
  const configResult = await loadConfig();
  if (!configResult.ok) return configResult;
  const config = configResult.value;

  const targets = name
    ? config.taps.filter((t) => t.name === name)
    : config.taps;

  if (name && targets.length === 0) {
    return err(
      new UserError(
        `Tap '${name}' is not configured.`,
        `Run 'skilltap tap list' to see configured taps.`,
      ),
    );
  }

  const counts: Record<string, number> = {};
  for (const tap of targets) {
    const dir = tapDir(tap.name);
    const pullResult = await pull(dir);
    if (!pullResult.ok) return pullResult;

    const tapResult = await loadTapJson(dir, tap.name);
    counts[tap.name] = tapResult.ok ? tapResult.value.skills.length : 0;
  }

  return ok(counts);
}

export async function loadTaps(): Promise<Result<TapEntry[], UserError>> {
  const configResult = await loadConfig();
  if (!configResult.ok) return configResult;
  const config = configResult.value;

  const entries: TapEntry[] = [];
  for (const tap of config.taps) {
    const dir = tapDir(tap.name);
    const tapResult = await loadTapJson(dir, tap.name);
    if (!tapResult.ok) {
      // Graceful degradation: skip invalid taps
      continue;
    }
    for (const skill of tapResult.value.skills) {
      entries.push({ tapName: tap.name, skill });
    }
  }

  return ok(entries);
}

export function searchTaps(skills: TapEntry[], query: string): TapEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return skills;
  return skills.filter(({ skill }) => {
    return (
      skill.name.toLowerCase().includes(q) ||
      skill.description.toLowerCase().includes(q) ||
      skill.tags.some((tag) => tag.toLowerCase().includes(q))
    );
  });
}

export async function initTap(name: string): Promise<Result<void, UserError>> {
  const dir = join(process.cwd(), name);
  try {
    await $`mkdir -p ${dir}`.quiet();
    await $`git -C ${dir} init`.quiet();
    // Set local git config so commit works without global config (e.g. CI)
    await $`git -C ${dir} config user.email "skilltap@localhost"`.quiet();
    await $`git -C ${dir} config user.name "skilltap"`.quiet();
    const tapJson = JSON.stringify(
      { name, description: "", skills: [] },
      null,
      2,
    );
    await Bun.write(join(dir, "tap.json"), tapJson);
    await $`git -C ${dir} add tap.json`.quiet();
    await $`git -C ${dir} commit -m "Initialize tap"`.quiet();
    return ok(undefined);
  } catch (e) {
    return err(
      new UserError(
        `Failed to initialize tap: ${e instanceof Error ? e.message : String(e)}`,
      ),
    );
  }
}
