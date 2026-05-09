import { ConfigSchema } from "../schemas/config";
import { checkAgents } from "./checks/agents";
import { checkCaptureCollisions } from "./checks/capture-collisions";
import { checkClaudeCodeOverlap } from "./checks/claude-code-overlap";
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
      // If every issue was fixed, the check is effectively passing.
      if (check.issues.length > 0 && check.issues.every((i) => i.fixed)) {
        check.fixed = true;
        const descriptions = check.issues
          .map((i) => i.fixDescription)
          .filter((d): d is string => Boolean(d));
        if (descriptions.length > 0) {
          check.fixDescription = descriptions.join("; ");
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

  // 4. installed (reads state.json; provides installed for later checks)
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

  // ── v2.x checks ────────────────────────────────────────────────────────────
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

  // 15. v0.x file orphans
  await emit(await checkV1Orphans(state, projectRoot));

  // 16. Capture collisions canary — skill in both state.skills[]
  // and a plugin's components[]. Should never fire after capture is wired.
  await emit(await checkCaptureCollisions(state));

  // 17. Claude Code plugin overlaps canary.
  await emit(await checkClaudeCodeOverlap(state));

  const hasFailure = checks.some((c) => c.status === "fail" && !c.fixed);
  return { ok: !hasFailure, checks };
}
