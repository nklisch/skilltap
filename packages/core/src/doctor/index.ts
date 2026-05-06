import { ConfigSchema } from "../schemas/config";
import { checkAgents } from "./checks/agents";
import { checkConfig } from "./checks/config";
import { checkDirs } from "./checks/directories";
import { checkGit } from "./checks/git";
import { checkInstalled } from "./checks/installed";
import { checkLockfileDrift } from "./checks/lockfile-drift";
import { checkManifestDrift } from "./checks/manifest-drift";
import { checkMcpConsistency } from "./checks/mcp-consistency";
import { checkNpm } from "./checks/npm";
import { checkPluginManifests } from "./checks/plugin-manifests";
import { checkSkills } from "./checks/skills";
import { checkStateV2 } from "./checks/state-v2";
import { checkSymlinks } from "./checks/symlinks";
import { checkTaps } from "./checks/taps";
import { checkV1Orphans } from "./checks/v1-orphans";
import type { DoctorCheck, DoctorOptions, DoctorResult } from "./types";

export type {
  DoctorCheck,
  DoctorIssue,
  DoctorOptions,
  DoctorResult,
} from "./types";

export async function runDoctor(
  options?: DoctorOptions,
): Promise<DoctorResult> {
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
  const { check: installedCheck, installed } =
    await checkInstalled(projectRoot);
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

  // ── v2.0 checks (Phase 36) ────────────────────────────────────────────────
  // 10. state.json — load + corruption recovery
  const { check: stateCheck, state } = await checkStateV2(projectRoot);
  await emit(stateCheck);

  // 11. manifest drift (skilltap.toml ↔ state.json)
  await emit(await checkManifestDrift(state, projectRoot));

  // 12. lockfile drift (skilltap.lock ↔ state.json)
  await emit(await checkLockfileDrift(state, projectRoot));

  // 13. plugin manifests (.skilltap/<name>.toml validity)
  await emit(await checkPluginManifests(projectRoot));

  // 14. MCP injection consistency (state ↔ agent configs)
  await emit(await checkMcpConsistency(state, projectRoot));

  // 15. v0.x file orphans (after canonical-store cutover in 31c-c-2d-1)
  await emit(await checkV1Orphans(state, projectRoot));

  const hasFailure = checks.some((c) => c.status === "fail");
  return { ok: !hasFailure, checks };
}
