import {
  type LockEntry,
  type Lockfile,
  LockfileSchema,
  loadLockfile,
  lockfileExists,
  saveLockfile,
} from "../../manifest";
import { recoverLockfile } from "../../manifest/recover";
import type { State } from "../../state/schema";
import type { DoctorCheck, DoctorIssue } from "../types";

export async function checkLockfileDrift(
  state: State | null,
  projectRoot?: string,
): Promise<DoctorCheck> {
  if (!state || !projectRoot) {
    return {
      name: "lockfile drift",
      status: "pass",
      detail: !state ? "n/a (no v2 state)" : "n/a (no project root)",
    };
  }
  if (!(await lockfileExists(projectRoot))) {
    return {
      name: "lockfile drift",
      status: "pass",
      detail: "n/a (no skilltap.lock)",
    };
  }

  const result = await loadLockfile(projectRoot);
  if (!result.ok) {
    return {
      name: "lockfile drift",
      status: "fail",
      issues: [
        {
          message: `Failed to load skilltap.lock: ${result.error.message}`,
          fixable: true,
          fixDescription:
            "backed up to skilltap.lock.bak, created fresh empty lockfile",
          fix: async () => {
            await recoverLockfile(projectRoot);
          },
        },
      ],
    };
  }
  const lockfile = result.value;

  const issues: DoctorIssue[] = [];

  const stateSourceMap = new Map<
    string,
    { kind: "skill" | "plugin"; ref: string | null; sha: string | null }
  >();
  for (const skill of state.skills) {
    if (skill.repo) {
      stateSourceMap.set(skill.repo, {
        kind: "skill",
        ref: skill.ref,
        sha: skill.sha,
      });
    }
  }
  for (const plugin of state.plugins) {
    if (plugin.repo) {
      stateSourceMap.set(plugin.repo, {
        kind: "plugin",
        ref: plugin.ref,
        sha: plugin.sha,
      });
    }
  }

  const lockedSources = new Set<string>();
  for (const entry of lockfile.skill) lockedSources.add(entry.source);
  for (const entry of lockfile.plugin) lockedSources.add(entry.source);

  for (const [source, info] of stateSourceMap) {
    if (!lockedSources.has(source) && info.ref) {
      issues.push({
        message: `${info.kind} '${source}' installed but missing from lockfile`,
        fixable: true,
        fixDescription: "regenerated lockfile entry from state",
        fix: async () => {
          const updated = await loadLockfile(projectRoot);
          if (!updated.ok) return;
          const newEntry: LockEntry = {
            source,
            ref: info.ref ?? "",
            sha: info.sha ?? undefined,
            range: info.ref ?? "*",
          };
          const next: Lockfile = LockfileSchema.parse({
            version: 1,
            skill:
              info.kind === "skill"
                ? [...updated.value.skill, newEntry]
                : updated.value.skill,
            plugin:
              info.kind === "plugin"
                ? [...updated.value.plugin, newEntry]
                : updated.value.plugin,
          });
          await saveLockfile(projectRoot, next);
        },
      });
    }
  }

  for (const entry of [...lockfile.skill, ...lockfile.plugin]) {
    const installed = stateSourceMap.get(entry.source);
    if (
      installed &&
      entry.sha &&
      installed.sha &&
      entry.sha !== installed.sha
    ) {
      issues.push({
        message: `${entry.source}: lockfile sha ${entry.sha.slice(0, 7)} differs from installed sha ${installed.sha.slice(0, 7)}`,
        fixable: false,
      });
    }
  }

  for (const entry of [...lockfile.skill, ...lockfile.plugin]) {
    if (!stateSourceMap.has(entry.source)) {
      issues.push({
        message: `${entry.source}: lockfile entry has no installed state`,
        fixable: false,
      });
    }
  }

  if (issues.length === 0) {
    return { name: "lockfile drift", status: "pass", detail: "in sync" };
  }
  const fixableCount = issues.filter((i) => i.fixable).length;
  return {
    name: "lockfile drift",
    status: "warn",
    detail: `${issues.length} drift item${issues.length === 1 ? "" : "s"} (${fixableCount} fixable)`,
    issues,
  };
}
