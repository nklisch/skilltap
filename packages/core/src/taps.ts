import { rm } from "node:fs/promises";
import { join } from "node:path";
import { $ } from "bun";
import { z } from "zod/v4";
import { getConfigDir, loadConfig, saveConfig } from "./config";
import { extractStderr } from "./shell";
import { checkGitInstalled, clone, pull } from "./git";
import type { RegistrySource } from "./registry";
import { detectTapType, fetchSkillList } from "./registry";
import type { Tap, TapSkill } from "./schemas/tap";
import { TapSchema } from "./schemas/tap";
import { err, type GitError, ok, type Result, UserError } from "./types";

export type TapEntry = { tapName: string; skill: TapSkill };

export type UpdateTapResult = {
  /** Git taps: skill counts after pull. */
  updated: Record<string, number>;
  /** HTTP tap names (always live, no update needed). */
  http: string[];
};

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

/** Map a registry source to a TapSkill repo string usable by the source adapter chain. */
function registrySourceToRepo(source: RegistrySource): string {
  switch (source.type) {
    case "git":
      return source.url;
    case "github":
      return source.repo;
    case "npm":
      return `npm:${source.package}`;
    case "url":
      return `url:${source.url}`;
  }
}

export type GitHubTapShorthand = { name: string; url: string };

const GH_LOCAL_PREFIXES = ["./", "/", "~/"];
const GH_URL_PROTOCOLS = ["https://", "http://", "git@", "ssh://", "npm:"];

/** Parse GitHub shorthand (owner/repo) into a tap name + clone URL. Returns null if not shorthand. */
export function parseGitHubTapShorthand(
  source: string,
): GitHubTapShorthand | null {
  let s = source;
  if (s.startsWith("github:")) s = s.slice("github:".length);
  else if (!s.includes("/")) return null;

  if (GH_URL_PROTOCOLS.some((p) => s.startsWith(p))) return null;
  if (GH_LOCAL_PREFIXES.some((p) => s.startsWith(p))) return null;

  // Strip @ref suffix (taps always clone HEAD)
  const atIdx = s.lastIndexOf("@");
  if (atIdx !== -1) s = s.slice(0, atIdx);

  const parts = s.split("/").filter(Boolean);
  if (parts.length !== 2) return null;

  const [owner, repo] = parts;
  return {
    name: repo!,
    url: `https://github.com/${owner}/${repo}.git`,
  };
}

export async function addTap(
  name: string,
  url: string,
  typeOverride?: "git" | "http",
): Promise<Result<{ skillCount: number; type: "git" | "http" }, UserError | GitError>> {
  const configResult = await loadConfig();
  if (!configResult.ok) return configResult;
  const config = configResult.value;

  if (config.taps.some((t) => t.name === name)) {
    return err(
      new UserError(
        `Tap '${name}' already exists.`,
        `Remove it first with 'skilltap tap remove ${name}'.`,
      ),
    );
  }

  // Auto-detect type if not specified
  const tapType = typeOverride ?? (await detectTapType(url));

  if (tapType === "http") {
    const listResult = await fetchSkillList(url, name, {});
    if (!listResult.ok) return listResult;

    config.taps.push({ name, url, type: "http" });
    const saveResult = await saveConfig(config);
    if (!saveResult.ok) return saveResult;

    return ok({ skillCount: listResult.value.skills.length, type: "http" });
  }

  // Git tap
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

  config.taps.push({ name, url, type: "git" });
  const saveResult = await saveConfig(config);
  if (!saveResult.ok) return saveResult;

  return ok({ skillCount: tapResult.value.skills.length, type: "git" });
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

  const tap = config.taps[idx]!;
  config.taps.splice(idx, 1);
  const saveResult = await saveConfig(config);
  if (!saveResult.ok) return saveResult;

  // Only clean up local directory for git taps
  if (tap.type !== "http") {
    await rm(tapDir(name), { recursive: true, force: true });
  }
  return ok(undefined);
}

export async function updateTap(
  name?: string,
): Promise<Result<UpdateTapResult, UserError | GitError>> {
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

  const updated: Record<string, number> = {};
  const http: string[] = [];

  for (const tap of targets) {
    if (tap.type === "http") {
      http.push(tap.name);
      continue;
    }

    // Git tap: pull and count skills
    const dir = tapDir(tap.name);
    const pullResult = await pull(dir);
    if (!pullResult.ok) return pullResult;

    const tapResult = await loadTapJson(dir, tap.name);
    updated[tap.name] = tapResult.ok ? tapResult.value.skills.length : 0;
  }

  return ok({ updated, http });
}

export async function loadTaps(): Promise<Result<TapEntry[], UserError>> {
  const configResult = await loadConfig();
  if (!configResult.ok) return configResult;
  const config = configResult.value;

  const entries: TapEntry[] = [];

  for (const tap of config.taps) {
    if (tap.type === "http") {
      // HTTP registry: fetch skills from API
      const auth = { token: tap.auth_token, envVar: tap.auth_env };
      const listResult = await fetchSkillList(tap.url, tap.name, auth);
      if (!listResult.ok) {
        // Graceful degradation: skip unreachable/invalid HTTP registries
        continue;
      }
      for (const skill of listResult.value.skills) {
        entries.push({
          tapName: tap.name,
          skill: {
            name: skill.name,
            description: skill.description,
            repo: registrySourceToRepo(skill.source),
            tags: skill.tags,
            trust: skill.trust,
          },
        });
      }
    } else {
      // Git tap: read local tap.json
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
  }

  return ok(entries);
}

export function searchTaps(skills: TapEntry[], query: string): TapEntry[] {
  const tokens = query
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);
  if (tokens.length === 0) return skills;

  const scored: Array<{ entry: TapEntry; score: number }> = [];

  for (const entry of skills) {
    const { skill } = entry;
    const name = skill.name.toLowerCase();
    const desc = skill.description.toLowerCase();
    const tags = skill.tags.map((t) => t.toLowerCase());

    let score = 0;
    let allMatch = true;

    for (const token of tokens) {
      const inName = name.includes(token);
      const inDesc = desc.includes(token);
      const inTags = tags.some((t) => t.includes(token));

      if (!inName && !inDesc && !inTags) {
        allMatch = false;
        break;
      }

      if (name === token) score += 8;
      else if (name.startsWith(token)) score += 4;
      else if (inName) score += 2;
      if (tags.some((t) => t === token)) score += 3;
      else if (inTags) score += 1;
      if (inDesc) score += 1;
    }

    if (allMatch) scored.push({ entry, score });
  }

  return scored.sort((a, b) => b.score - a.score).map((s) => s.entry);
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
      new UserError(`Failed to initialize tap: ${extractStderr(e)}`),
    );
  }
}
