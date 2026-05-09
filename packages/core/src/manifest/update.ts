import { ok, type Result, type UserError } from "../types";
import { loadManifest, manifestExists } from "./load";
import { loadLockfile, saveLockfile } from "./lockfile";
import { saveManifest } from "./save";
import {
  type LockEntry,
  type Lockfile,
  type LockfileMcpEntry,
  LockfileSchema,
  type ManifestEntry,
  type ManifestMcpEntry,
  type ProjectManifest,
} from "./schemas";

export interface ManifestUpdateInput {
  /** Source string preferred — falls back to record.repo. */
  source: string;
  /** Branch/tag/ref installed (the reference for the lockfile entry). */
  ref: string | null;
  /** Resolved sha at install time. */
  sha: string | null;
  /** Range to record in the manifest. Defaults to "*". */
  range?: string;
}

// Convert an install record's repo URL into the canonical manifest source key.
// - `https://github.com/owner/repo[.git]` → `github:owner/repo`
// - `git@github.com:owner/repo[.git]` → `github:owner/repo`
// - `npm:@scope/name[@version]` → unchanged
// - Everything else: passthrough.
export function canonicalizeSourceKey(repoOrSource: string): string {
  // SSH form: git@host:owner/repo[.git]
  const sshMatch = /^git@([^:]+):([^/]+)\/([^/.]+?)(?:\.git)?$/.exec(
    repoOrSource,
  );
  if (sshMatch && sshMatch[1] === "github.com") {
    return `github:${sshMatch[2]}/${sshMatch[3]}`;
  }

  // HTTPS form: https://github.com/owner/repo[.git]
  const httpsMatch =
    /^https?:\/\/github\.com\/([^/]+)\/([^/]+?)(?:\.git)?$/.exec(repoOrSource);
  if (httpsMatch) {
    return `github:${httpsMatch[1]}/${httpsMatch[2]}`;
  }

  return repoOrSource;
}

// Append (or replace) a skill entry in skilltap.toml + skilltap.lock at the
// project root. No-op if no manifest exists. Manifest-write failures are
// converted to silent ok() — the install/plugin-install caller already
// succeeded; we don't roll back over a manifest hiccup.
export async function addSkillToManifest(
  projectRoot: string,
  input: ManifestUpdateInput,
): Promise<Result<void, UserError>> {
  return updateManifestEntry(projectRoot, input, "skills");
}

export async function addPluginToManifest(
  projectRoot: string,
  input: ManifestUpdateInput,
): Promise<Result<void, UserError>> {
  return updateManifestEntry(projectRoot, input, "plugins");
}

// Remove a skill or plugin entry from skilltap.toml + skilltap.lock.
// No-op without skilltap.toml. Failures are silenced (non-fatal — the
// removal already succeeded at the install-state level).
export async function removeSkillFromManifest(
  projectRoot: string,
  source: string,
): Promise<Result<void, UserError>> {
  return removeManifestEntry(projectRoot, source, "skills");
}

export async function removePluginFromManifest(
  projectRoot: string,
  source: string,
): Promise<Result<void, UserError>> {
  return removeManifestEntry(projectRoot, source, "plugins");
}

async function removeManifestEntry(
  projectRoot: string,
  source: string,
  kind: "skills" | "plugins",
): Promise<Result<void, UserError>> {
  if (!(await manifestExists(projectRoot))) return ok(undefined);

  const sourceKey = canonicalizeSourceKey(source);

  const manifestResult = await loadManifest(projectRoot);
  if (!manifestResult.ok) return ok(undefined);
  const manifest = manifestResult.value;
  if (!(sourceKey in manifest[kind])) {
    // Not in manifest — could be a global install or a different canonical key.
    // Still drop from lockfile if present.
  } else {
    const { [sourceKey]: _, ...rest } = manifest[kind];
    const updated: ProjectManifest = {
      ...manifest,
      [kind]: rest,
    };
    const saveManifestResult = await saveManifest(projectRoot, updated);
    if (!saveManifestResult.ok) return ok(undefined);
  }

  const lockfileResult = await loadLockfile(projectRoot);
  if (!lockfileResult.ok) return ok(undefined);
  const lockfile = lockfileResult.value;

  const targetArray: "skill" | "plugin" =
    kind === "skills" ? "skill" : "plugin";
  const filtered = lockfile[targetArray].filter((e) => e.source !== sourceKey);
  if (filtered.length === lockfile[targetArray].length) return ok(undefined);

  const nextLockfile: Lockfile = LockfileSchema.parse({
    version: 1,
    skill: targetArray === "skill" ? filtered : lockfile.skill,
    plugin: targetArray === "plugin" ? filtered : lockfile.plugin,
  });
  await saveLockfile(projectRoot, nextLockfile);
  return ok(undefined);
}

// ---------------------------------------------------------------------------
// MCP manifest + lockfile writers (Unit 1.14)
// ---------------------------------------------------------------------------

export interface ManifestMcpUpdateInput {
  name: string;
  source: string;
  ref?: string;
  also?: string[];
}

export interface LockfileMcpUpdateInput {
  name: string;
  source: string;
  ref: string;
  sha: string;
  also?: string[];
}

export async function addMcpToManifest(
  projectRoot: string,
  input: ManifestMcpUpdateInput,
): Promise<Result<void, UserError>> {
  if (!(await manifestExists(projectRoot))) return ok(undefined);

  const manifestResult = await loadManifest(projectRoot);
  if (!manifestResult.ok) return ok(undefined);
  const manifest = manifestResult.value;

  const newEntry: ManifestMcpEntry = {
    name: input.name,
    source: input.source,
    ref: input.ref ?? "main",
    also: input.also ?? [],
  };

  const existingIdx = manifest.mcps.findIndex((m) => m.name === input.name);
  const nextMcps =
    existingIdx === -1
      ? [...manifest.mcps, newEntry]
      : manifest.mcps.map((m, i) => (i === existingIdx ? newEntry : m));

  const updated: ProjectManifest = { ...manifest, mcps: nextMcps };
  await saveManifest(projectRoot, updated);
  return ok(undefined);
}

export async function removeMcpFromManifest(
  projectRoot: string,
  name: string,
): Promise<Result<void, UserError>> {
  if (!(await manifestExists(projectRoot))) return ok(undefined);

  const manifestResult = await loadManifest(projectRoot);
  if (!manifestResult.ok) return ok(undefined);
  const manifest = manifestResult.value;

  const filtered = manifest.mcps.filter((m) => m.name !== name);
  if (filtered.length === manifest.mcps.length) return ok(undefined);

  const updated: ProjectManifest = { ...manifest, mcps: filtered };
  await saveManifest(projectRoot, updated);
  return ok(undefined);
}

export async function addMcpToLockfile(
  projectRoot: string,
  input: LockfileMcpUpdateInput,
): Promise<Result<void, UserError>> {
  const lockfileResult = await loadLockfile(projectRoot);
  if (!lockfileResult.ok) return ok(undefined);
  const lockfile = lockfileResult.value;

  const newEntry: LockfileMcpEntry = {
    name: input.name,
    source: input.source,
    ref: input.ref,
    sha: input.sha,
    also: input.also ?? [],
  };

  const existingIdx = lockfile.mcps.findIndex((m) => m.name === input.name);
  const nextMcps =
    existingIdx === -1
      ? [...lockfile.mcps, newEntry]
      : lockfile.mcps.map((m, i) => (i === existingIdx ? newEntry : m));

  const next: Lockfile = LockfileSchema.parse({
    version: 1,
    skill: lockfile.skill,
    plugin: lockfile.plugin,
    mcps: nextMcps,
  });
  await saveLockfile(projectRoot, next);
  return ok(undefined);
}

export async function removeMcpFromLockfile(
  projectRoot: string,
  name: string,
): Promise<Result<void, UserError>> {
  const lockfileResult = await loadLockfile(projectRoot);
  if (!lockfileResult.ok) return ok(undefined);
  const lockfile = lockfileResult.value;

  const filtered = lockfile.mcps.filter((m) => m.name !== name);
  if (filtered.length === lockfile.mcps.length) return ok(undefined);

  const next: Lockfile = LockfileSchema.parse({
    version: 1,
    skill: lockfile.skill,
    plugin: lockfile.plugin,
    mcps: filtered,
  });
  await saveLockfile(projectRoot, next);
  return ok(undefined);
}

// ---------------------------------------------------------------------------

async function updateManifestEntry(
  projectRoot: string,
  input: ManifestUpdateInput,
  kind: "skills" | "plugins",
): Promise<Result<void, UserError>> {
  if (!(await manifestExists(projectRoot))) return ok(undefined);

  const sourceKey = canonicalizeSourceKey(input.source);
  const range = input.range ?? "*";

  const manifestResult = await loadManifest(projectRoot);
  if (!manifestResult.ok) return ok(undefined);
  const manifest = manifestResult.value;
  const updated: ProjectManifest = {
    ...manifest,
    [kind]: {
      ...manifest[kind],
      [sourceKey]: range as ManifestEntry,
    },
  };
  const saveManifestResult = await saveManifest(projectRoot, updated);
  if (!saveManifestResult.ok) return ok(undefined);

  const lockfileResult = await loadLockfile(projectRoot);
  if (!lockfileResult.ok) return ok(undefined);
  const lockfile = lockfileResult.value;

  const targetArray: "skill" | "plugin" =
    kind === "skills" ? "skill" : "plugin";
  const existing = lockfile[targetArray];
  const existingIdx = existing.findIndex((e) => e.source === sourceKey);
  const newEntry: LockEntry = {
    source: sourceKey,
    ref: input.ref ?? "",
    sha: input.sha ?? undefined,
    range,
  };
  const nextEntries =
    existingIdx === -1
      ? [...existing, newEntry]
      : existing.map((e, i) => (i === existingIdx ? newEntry : e));
  const nextLockfile: Lockfile = LockfileSchema.parse({
    version: 1,
    skill: targetArray === "skill" ? nextEntries : lockfile.skill,
    plugin: targetArray === "plugin" ? nextEntries : lockfile.plugin,
  });

  await saveLockfile(projectRoot, nextLockfile);
  return ok(undefined);
}
