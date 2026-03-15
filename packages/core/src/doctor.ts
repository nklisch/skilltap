import {
  copyFile,
  lstat,
  mkdir,
  readdir,
  readlink,
  stat,
  symlink,
  unlink,
  writeFile,
} from "node:fs/promises";
import { join } from "node:path";
import { $ } from "bun";
import { parse } from "smol-toml";
import { z } from "zod/v4";
import { detectAgents } from "./agents/detect";
import { getConfigDir, loadInstalled, migrateSecurityConfig, saveInstalled } from "./config";
import { globalBase } from "./fs";
import { clone } from "./git";
import { skillInstallDir } from "./paths";
import type { Config } from "./schemas/config";
import { ConfigSchema } from "./schemas/config";
import type { InstalledJson } from "./schemas/installed";
import { InstalledJsonSchema } from "./schemas/installed";
import { TapSchema } from "./schemas/tap";
import { AGENT_PATHS } from "./symlink";
import { BUILTIN_TAP } from "./taps";

export interface DoctorIssue {
  message: string;
  fixable: boolean;
  /** Describes what the fix action did, shown after ` — ` when fixed. */
  fixDescription?: string;
  fix?: () => Promise<void>;
  /** Set to true after fix() completes successfully. */
  fixed?: boolean;
}

export interface DoctorCheck {
  name: string;
  status: "pass" | "warn" | "fail";
  detail?: string;
  issues?: DoctorIssue[];
  /** Informational per-item status lines shown after issues (e.g. per-tap health). */
  info?: string[];
}

export interface DoctorResult {
  ok: boolean;
  checks: DoctorCheck[];
}

export interface DoctorOptions {
  fix?: boolean;
  projectRoot?: string;
  onCheck?: (check: DoctorCheck) => void;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async function resolvedDirExists(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isDirectory();
  } catch {
    return false;
  }
}

async function fileExists(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isFile();
  } catch {
    return false;
  }
}

async function isSymlinkAt(path: string): Promise<boolean> {
  try {
    return (await lstat(path)).isSymbolicLink();
  } catch {
    return false;
  }
}

// ─── Check 1: Git ─────────────────────────────────────────────────────────────

async function checkGit(): Promise<DoctorCheck> {
  let gitPath: string;
  try {
    gitPath = await $`which git`.quiet().then((r) => r.stdout.toString().trim());
  } catch {
    return {
      name: "git",
      status: "fail",
      issues: [
        {
          message: "git not found on PATH. Install git: https://git-scm.com",
          fixable: false,
        },
      ],
    };
  }

  let versionStr: string;
  try {
    versionStr = await $`git --version`
      .quiet()
      .then((r) => r.stdout.toString().trim());
  } catch {
    return {
      name: "git",
      status: "fail",
      issues: [
        {
          message: "git not found on PATH. Install git: https://git-scm.com",
          fixable: false,
        },
      ],
    };
  }

  const match = versionStr.match(/(\d+)\.(\d+)\.(\d+)/);
  const major = match ? parseInt(match[1]!, 10) : 0;
  const minor = match ? parseInt(match[2]!, 10) : 0;
  const patch = match ? parseInt(match[3]!, 10) : 0;
  const versionTag = match ? `${major}.${minor}.${patch}` : versionStr;
  const detail = `${gitPath} (${versionTag})`;

  if (major < 2 || (major === 2 && minor < 25)) {
    return {
      name: "git",
      status: "warn",
      detail,
      issues: [
        {
          message: `git 2.25+ recommended (found ${versionTag}). Shallow clone --filter may not work.`,
          fixable: false,
        },
      ],
    };
  }

  return { name: "git", status: "pass", detail };
}

// ─── Check 2: Config ─────────────────────────────────────────────────────────

async function checkConfig(): Promise<{
  check: DoctorCheck;
  config: Config | null;
}> {
  const configDir = getConfigDir();
  const configFile = join(configDir, "config.toml");

  if (!(await fileExists(configFile))) {
    const check: DoctorCheck = {
      name: "config",
      status: "warn",
      issues: [
        {
          message: "No config.toml found. Run 'skilltap config' to create one.",
          fixable: true,
          fixDescription: "created default config",
          fix: async () => {
            const { loadConfig } = await import("./config");
            await loadConfig();
          },
        },
      ],
    };
    return { check, config: null };
  }

  let text: string;
  try {
    text = await Bun.file(configFile).text();
  } catch (e) {
    return {
      check: {
        name: "config",
        status: "fail",
        issues: [{ message: `Cannot read config.toml: ${e}`, fixable: false }],
      },
      config: null,
    };
  }

  let raw: unknown;
  try {
    raw = parse(text);
  } catch (e) {
    return {
      check: {
        name: "config",
        status: "fail",
        issues: [
          {
            message: `config.toml is invalid TOML: ${e}`,
            fixable: false,
          },
        ],
      },
      config: null,
    };
  }

  const migrated = migrateSecurityConfig(raw as Record<string, unknown>);
  const result = ConfigSchema.safeParse(migrated);
  if (!result.success) {
    return {
      check: {
        name: "config",
        status: "fail",
        issues: [
          {
            message: `config.toml has invalid values: ${z.prettifyError(result.error)}`,
            fixable: false,
          },
        ],
      },
      config: null,
    };
  }

  return {
    check: { name: "config", status: "pass", detail: configFile },
    config: result.data,
  };
}

// ─── Check 3: Directories ─────────────────────────────────────────────────────

async function checkDirs(): Promise<DoctorCheck> {
  const configDir = getConfigDir();
  const issues: DoctorIssue[] = [];

  const required = [
    configDir,
    join(configDir, "cache"),
    join(configDir, "taps"),
    join(globalBase(), ".agents", "skills"),
  ];

  for (const dir of required) {
    if (!(await resolvedDirExists(dir))) {
      issues.push({
        message: `Missing directory: ${dir}`,
        fixable: true,
        fixDescription: `created ${dir}`,
        fix: async () => {
          await mkdir(dir, { recursive: true });
        },
      });
    }
  }

  if (issues.length === 0) {
    return { name: "dirs", status: "pass", detail: configDir };
  }
  return { name: "dirs", status: "warn", issues };
}

// ─── Check 4: installed.json ──────────────────────────────────────────────────

async function readInstalledFile(
  file: string,
  label: string,
  issues: DoctorIssue[],
): Promise<InstalledJson | null> {
  if (!(await fileExists(file))) return null;

  let raw: unknown;
  try {
    raw = await Bun.file(file).json();
  } catch (e) {
    const backupFile = `${file}.bak`;
    const backupName = `${label}.bak`;
    issues.push({
      message: `${label} is corrupt: ${e}`,
      fixable: true,
      fixDescription: `backed up to ${backupName}, created fresh`,
      fix: async () => {
        await copyFile(file, backupFile).catch(() => {});
        await writeFile(file, JSON.stringify({ version: 1, skills: [] }, null, 2));
      },
    });
    return null;
  }

  const result = InstalledJsonSchema.safeParse(raw);
  if (!result.success) {
    const backupFile = `${file}.bak`;
    const backupName = `${label}.bak`;
    issues.push({
      message: `${label} is invalid: ${z.prettifyError(result.error)}`,
      fixable: true,
      fixDescription: `backed up to ${backupName}, created fresh`,
      fix: async () => {
        await copyFile(file, backupFile).catch(() => {});
        await writeFile(file, JSON.stringify({ version: 1, skills: [] }, null, 2));
      },
    });
    return null;
  }

  return result.data;
}

async function checkInstalled(projectRoot?: string): Promise<{
  check: DoctorCheck;
  installed: InstalledJson | null;
}> {
  const globalFile = join(getConfigDir(), "installed.json");
  const issues: DoctorIssue[] = [];

  const globalInstalled = await readInstalledFile(globalFile, "installed.json", issues);
  const projectInstalled = projectRoot
    ? await readInstalledFile(
        join(projectRoot, ".agents", "installed.json"),
        ".agents/installed.json",
        issues,
      )
    : null;

  const allSkills = [
    ...(globalInstalled?.skills ?? []),
    ...(projectInstalled?.skills ?? []),
  ];
  const merged: InstalledJson = { version: 1 as const, skills: allSkills };

  if (issues.length > 0) {
    return { check: { name: "installed", status: "fail", issues }, installed: merged };
  }

  const globalCount = globalInstalled?.skills.length ?? 0;
  const projectCount = projectInstalled?.skills.length ?? 0;
  const total = allSkills.length;

  let detail: string;
  if (!globalInstalled && !projectInstalled) {
    detail = "0 skills (no installed.json)";
  } else if (projectInstalled !== null) {
    detail = `${total} skill${total === 1 ? "" : "s"} (${globalCount} global, ${projectCount} project)`;
  } else {
    detail = `${total} skill${total === 1 ? "" : "s"}`;
  }

  return { check: { name: "installed", status: "pass", detail }, installed: merged };
}

// ─── Check 5: Skills Integrity ────────────────────────────────────────────────

async function checkSkills(installed: InstalledJson, projectRoot?: string): Promise<DoctorCheck> {
  const issues: DoctorIssue[] = [];
  const globalTracked = new Set<string>();
  const projectTracked = new Set<string>();

  for (const skill of installed.skills) {
    if (skill.scope === "project") {
      projectTracked.add(skill.name);
    } else if (skill.scope === "linked") {
      // Linked skills create a symlink in .agents/skills/ — track in the appropriate bucket
      if (projectRoot && skill.path?.startsWith(join(projectRoot, ".agents"))) {
        projectTracked.add(skill.name);
      } else {
        globalTracked.add(skill.name);
      }
    } else {
      globalTracked.add(skill.name);
    }

    if (skill.scope === "linked") {
      if (skill.path && !(await resolvedDirExists(skill.path))) {
        const skillName = skill.name;
        issues.push({
          message: `${skillName}: symlink target ${skill.path} does not exist`,
          fixable: true,
          fixDescription: `removed from installed.json`,
          fix: async () => {
            const r = await loadInstalled();
            if (!r.ok) return;
            await saveInstalled({
              ...r.value,
              skills: r.value.skills.filter((s) => s.name !== skillName),
            });
          },
        });
      }
      continue;
    }

    const isProject = skill.scope === "project" && !!projectRoot;
    const installDir = isProject
      ? skillInstallDir(skill.name, "project", projectRoot)
      : skillInstallDir(skill.name, "global");

    if (!(await resolvedDirExists(installDir))) {
      const skillName = skill.name;
      const skillScope = skill.scope as "global" | "project";
      const capturedRoot = projectRoot;
      issues.push({
        message: `${skillName}: recorded in installed.json but directory missing at ${installDir}`,
        fixable: true,
        fixDescription: `removed from installed.json`,
        fix: async () => {
          const effectiveRoot = skillScope === "project" ? capturedRoot : undefined;
          const r = await loadInstalled(effectiveRoot);
          if (!r.ok) return;
          await saveInstalled(
            { ...r.value, skills: r.value.skills.filter((s) => s.name !== skillName) },
            effectiveRoot,
          );
        },
      });
    }
  }

  // Global orphan scan
  const globalSkillsDir = join(globalBase(), ".agents", "skills");
  if (await resolvedDirExists(globalSkillsDir)) {
    try {
      const entries = await readdir(globalSkillsDir, { withFileTypes: true });
      for (const entry of entries) {
        if (!entry.isDirectory() && !entry.isSymbolicLink()) continue;
        if (!globalTracked.has(entry.name)) {
          issues.push({
            message: `${entry.name}: directory exists at ${join(globalSkillsDir, entry.name)} but not tracked in installed.json`,
            fixable: false,
          });
        }
      }
    } catch {
      // ignore
    }
  }

  // Project orphan scan (only when there are project-tracked skills)
  if (projectRoot && projectTracked.size > 0) {
    const projectSkillsDir = join(projectRoot, ".agents", "skills");
    if (await resolvedDirExists(projectSkillsDir)) {
      try {
        const entries = await readdir(projectSkillsDir, { withFileTypes: true });
        for (const entry of entries) {
          if (!entry.isDirectory() && !entry.isSymbolicLink()) continue;
          if (!projectTracked.has(entry.name)) {
            issues.push({
              message: `${entry.name}: directory exists at ${join(projectSkillsDir, entry.name)} but not tracked in installed.json`,
              fixable: false,
            });
          }
        }
      } catch {
        // ignore
      }
    }
  }

  const total = installed.skills.length;
  const missing = issues.filter((i) => i.fixable).length;
  const onDisk = total - missing;

  if (issues.length === 0) {
    return {
      name: "skills",
      status: "pass",
      detail: `${total} installed, ${total} on disk`,
    };
  }
  return {
    name: "skills",
    status: "warn",
    detail: `${total} installed, ${onDisk} on disk`,
    issues,
  };
}

// ─── Check 6: Agent Symlinks ──────────────────────────────────────────────────

async function checkSymlinks(installed: InstalledJson, projectRoot?: string): Promise<DoctorCheck> {
  const issues: DoctorIssue[] = [];
  let total = 0;
  let valid = 0;

  for (const skill of installed.skills) {
    if (skill.also.length === 0) continue;

    const isLinked = skill.scope === "linked";
    const isProject =
      (skill.scope === "project" && !!projectRoot) ||
      (isLinked && !!projectRoot && !!skill.path?.startsWith(join(projectRoot, ".agents")));
    const expectedTarget = isLinked
      ? (skill.path ?? skillInstallDir(skill.name, "global"))
      : isProject
        ? skillInstallDir(skill.name, "project", projectRoot)
        : skillInstallDir(skill.name, "global");
    const base = isProject ? projectRoot! : globalBase();

    for (const agent of skill.also) {
      const agentRelDir = AGENT_PATHS[agent];
      if (!agentRelDir) continue;

      const linkPath = join(base, agentRelDir, skill.name);
      total++;

      const isLink = await isSymlinkAt(linkPath);
      if (!isLink) {
        // Check if skill itself still exists
        const skillExists = await resolvedDirExists(expectedTarget);
        const fixDesc = skillExists
          ? "recreated symlink"
          : "removed (skill no longer installed)";
        issues.push({
          message: `${skill.name}: missing symlink at ${linkPath}`,
          fixable: true,
          fixDescription: fixDesc,
          fix: skillExists
            ? async () => {
                await mkdir(join(linkPath, ".."), { recursive: true });
                await symlink(expectedTarget, linkPath, "dir").catch(() => {});
              }
            : async () => {
                // Nothing to do — orphan record cleanup handled by checkSkills fix
              },
        });
        continue;
      }

      let target: string | null = null;
      try {
        target = await readlink(linkPath);
      } catch {
        // ignore
      }

      if (target !== expectedTarget) {
        issues.push({
          message: `${skill.name}: symlink at ${linkPath} points to wrong target`,
          fixable: true,
          fixDescription: "recreated symlink",
          fix: async () => {
            await unlink(linkPath).catch(() => {});
            await mkdir(join(linkPath, ".."), { recursive: true });
            await symlink(expectedTarget, linkPath, "dir").catch(() => {});
          },
        });
      } else {
        valid++;
      }
    }
  }

  if (issues.length === 0) {
    return {
      name: "symlinks",
      status: "pass",
      detail: `${total} symlinks, ${valid} valid`,
    };
  }
  return {
    name: "symlinks",
    status: "warn",
    detail: `${total} symlinks, ${valid} valid`,
    issues,
  };
}

// ─── Check 7: Taps ───────────────────────────────────────────────────────────

async function checkTaps(config: Config): Promise<DoctorCheck> {
  const issues: DoctorIssue[] = [];
  const info: string[] = [];
  let validCount = 0;

  const hasBuiltin = config.builtin_tap !== false;
  const allTaps: Array<{ name: string; url: string; type: "git" | "http" | "builtin" }> = [];

  if (hasBuiltin) {
    allTaps.push({ name: BUILTIN_TAP.name, url: BUILTIN_TAP.url, type: "builtin" });
  }
  for (const tap of config.taps) {
    allTaps.push({ name: tap.name, url: tap.url, type: tap.type });
  }

  if (allTaps.length === 0) {
    return { name: "taps", status: "pass", detail: "0 configured" };
  }

  for (const tap of allTaps) {
    if (tap.type === "http") {
      validCount++;
      info.push(`${tap.name} (http): ok`);
      continue;
    }

    const dir = join(getConfigDir(), "taps", tap.name);
    const label = tap.type === "builtin" ? `${tap.name} (built-in)` : tap.name;

    if (!(await resolvedDirExists(dir))) {
      const tapUrl = tap.url;
      issues.push({
        message: `tap '${tap.name}': directory missing. Run 'skilltap tap update ${tap.name}' to re-clone.`,
        fixable: true,
        fixDescription: "re-cloned tap",
        fix: async () => {
          await clone(tapUrl, dir, { depth: 1 });
        },
      });
      continue;
    }

    const tapJsonFile = join(dir, "tap.json");
    if (!(await fileExists(tapJsonFile))) {
      issues.push({
        message: `tap '${tap.name}': tap.json is missing`,
        fixable: false,
      });
      continue;
    }

    let tapRaw: unknown;
    try {
      tapRaw = await Bun.file(tapJsonFile).json();
    } catch (e) {
      issues.push({
        message: `tap '${tap.name}': tap.json is invalid JSON: ${e}`,
        fixable: false,
      });
      continue;
    }

    const tapResult = TapSchema.safeParse(tapRaw);
    if (!tapResult.success) {
      issues.push({
        message: `tap '${tap.name}': tap.json is invalid: ${z.prettifyError(tapResult.error)}`,
        fixable: false,
      });
      continue;
    }

    const gitDir = join(dir, ".git");
    if (!(await resolvedDirExists(gitDir))) {
      issues.push({
        message: `tap '${tap.name}': .git directory missing (not a git repo)`,
        fixable: false,
      });
      continue;
    }

    validCount++;
    info.push(`${label}: ok (${tapResult.data.skills.length} skills)`);
  }

  const total = allTaps.length;

  if (issues.length === 0) {
    return {
      name: "taps",
      status: "pass",
      detail: `${total} configured, ${validCount} valid`,
      info,
    };
  }
  return {
    name: "taps",
    status: "warn",
    detail: `${total} configured, ${validCount} valid`,
    issues,
    info,
  };
}

// ─── Check 8: Agent CLIs ──────────────────────────────────────────────────────

async function checkAgents(config: Config): Promise<DoctorCheck> {
  const available = await detectAgents();
  const configuredAgent = config.security.agent_cli;

  if (configuredAgent) {
    const isAbsPath = configuredAgent.startsWith("/");
    if (isAbsPath) {
      const exists = await fileExists(configuredAgent);
      if (!exists) {
        return {
          name: "agents",
          status: "warn",
          detail:
            available.length > 0
              ? `${available.length} detected`
              : "none detected",
          issues: [
            {
              message: `Configured agent '${configuredAgent}' not found on disk. Semantic scan will fail.`,
              fixable: false,
            },
          ],
        };
      }
    } else {
      const found = available.find(
        (a) =>
          a.cliName === configuredAgent ||
          a.name.toLowerCase() === configuredAgent,
      );
      if (!found) {
        return {
          name: "agents",
          status: "warn",
          detail:
            available.length > 0
              ? `${available.length} detected`
              : "none detected",
          issues: [
            {
              message: `Configured agent '${configuredAgent}' not found on PATH. Semantic scan will fail.`,
              fixable: false,
            },
          ],
        };
      }
    }
  }

  if (available.length === 0) {
    return {
      name: "agents",
      status: "pass",
      detail: "none detected (semantic scanning unavailable)",
    };
  }

  const names = available.map((a) => a.cliName).join(", ");
  return {
    name: "agents",
    status: "pass",
    detail: `${available.length} detected (${names})`,
  };
}

// ─── Check 9: npm (conditional) ───────────────────────────────────────────────

async function checkNpm(installed: InstalledJson): Promise<DoctorCheck | null> {
  const hasNpmSkills = installed.skills.some((s) => s.repo?.startsWith("npm:"));
  if (!hasNpmSkills) return null;

  let npmPath: string;
  try {
    npmPath = await $`which npm`
      .quiet()
      .then((r) => r.stdout.toString().trim());
  } catch {
    return {
      name: "npm",
      status: "warn",
      issues: [
        {
          message: "npm not found. Install Node.js for npm skill support.",
          fixable: false,
        },
      ],
    };
  }

  let version: string;
  try {
    version = await $`npm --version`
      .quiet()
      .then((r) => r.stdout.toString().trim());
  } catch {
    return {
      name: "npm",
      status: "warn",
      issues: [{ message: "npm --version failed", fixable: false }],
    };
  }

  const issues: DoctorIssue[] = [];

  try {
    await $`npm ping`.quiet();
  } catch {
    issues.push({
      message:
        "npm registry is not reachable. Check your network or registry config.",
      fixable: false,
    });
  }

  let whoami: string | null = null;
  try {
    whoami = await $`npm whoami`
      .quiet()
      .then((r) => r.stdout.toString().trim());
  } catch {
    issues.push({
      message: "Not logged in to npm. Run 'npm login' if you need to publish.",
      fixable: false,
    });
  }

  const detail = whoami
    ? `${npmPath} (${version}) — logged in as ${whoami}`
    : `${npmPath} (${version})`;

  if (issues.length === 0) {
    return { name: "npm", status: "pass", detail };
  }
  return { name: "npm", status: "warn", detail, issues };
}

// ─── Orchestrator ─────────────────────────────────────────────────────────────

export async function runDoctor(options?: DoctorOptions): Promise<DoctorResult> {
  const fix = options?.fix ?? false;
  const onCheck = options?.onCheck;
  const projectRoot = options?.projectRoot;
  const checks: DoctorCheck[] = [];

  async function emit(check: DoctorCheck): Promise<DoctorCheck> {
    if (fix && check.issues) {
      for (const issue of check.issues) {
        if (issue.fixable && issue.fix) {
          try {
            await issue.fix();
            issue.fixed = true;
          } catch {
            // fix failed — leave fixed = false
          }
        }
      }
    }
    onCheck?.(check);
    checks.push(check);
    return check;
  }

  // 1. Git
  await emit(await checkGit());

  // 2. Config (provides config for later checks)
  const { check: configCheck, config } = await checkConfig();
  await emit(configCheck);

  // 3. Dirs
  await emit(await checkDirs());

  // 4. installed.json (provides installed for later checks)
  const { check: installedCheck, installed } = await checkInstalled(projectRoot);
  await emit(installedCheck);

  const safeInstalled = installed ?? { version: 1 as const, skills: [] };
  const safeConfig = config ?? ConfigSchema.parse({});

  // 5. Skills integrity
  await emit(await checkSkills(safeInstalled, projectRoot));

  // 6. Agent symlinks
  await emit(await checkSymlinks(safeInstalled, projectRoot));

  // 7. Taps
  await emit(await checkTaps(safeConfig));

  // 8. Agent CLIs
  await emit(await checkAgents(safeConfig));

  // 9. npm (conditional)
  const npmCheck = await checkNpm(safeInstalled);
  if (npmCheck) await emit(npmCheck);

  const hasFailure = checks.some((c) => c.status === "fail");
  return { ok: !hasFailure, checks };
}
