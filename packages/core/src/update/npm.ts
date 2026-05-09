import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { $ } from "bun";
import { makeTmpDir, removeTmpDir } from "../fs";
import {
  downloadAndExtract,
  fetchPackageMetadata,
  parseNpmSource,
  resolveVersion,
} from "../npm-registry";
import { currentSkillDir } from "../paths";
import type { InstalledSkill } from "../schemas/installed";
import { scanStatic } from "../security";
import { wrapShell } from "../shell";
import { parseGitHubRepo } from "../trust";
import type { Result, NetworkError, ScanError, UserError } from "../types";
import { ok } from "../types";
import {
  patchRecord,
  refreshAgentSymlinks,
  runUpdateSemanticScan,
  shouldSkipUpdate,
  skipSkill,
} from "./shared";
import type { ResolveTrustFn, UpdateOptions, UpdateResult } from "./types";

/** Handle updates for npm-sourced skills (version comparison instead of git SHA). */
export async function updateNpmSkill(
  record: InstalledSkill,
  installed: { skills: InstalledSkill[] },
  options: UpdateOptions,
  result: UpdateResult,
  _resolveTrust: ResolveTrustFn,
): Promise<Result<void, UserError | NetworkError | ScanError>> {
  // biome-ignore lint/style/noNonNullAssertion: caller checks record.repo?.startsWith("npm:")
  const { name: packageName } = parseNpmSource(record.repo!);

  const metaResult = await fetchPackageMetadata(packageName);
  if (!metaResult.ok) {
    // Network failure — skip gracefully rather than hard-failing the whole update
    return skipSkill(result, options, record.name);
  }

  const versionResult = resolveVersion(metaResult.value, "latest");
  if (!versionResult.ok) {
    return skipSkill(result, options, record.name);
  }

  const latestVersion = versionResult.value.version;

  if (record.ref === latestVersion && !options.force) {
    await refreshAgentSymlinks(record, options.projectRoot);
    result.upToDate.push(record.name);
    options.onProgress?.(record.name, "upToDate");
    return ok(undefined);
  }

  const tmpResult = await makeTmpDir();
  if (!tmpResult.ok) return tmpResult;
  const tmpDir = tmpResult.value;

  try {
    const info = versionResult.value;
    const extractResult = await downloadAndExtract(
      info.dist.tarball,
      tmpDir,
      info.dist.integrity,
    );
    if (!extractResult.ok) {
      return skipSkill(result, options, record.name);
    }

    const pkgDir = extractResult.value;
    // Standalone: path is null → use the whole package dir
    // Multi-skill: path is relative within the package (e.g. "skills/skill-a")
    const newSkillDir = record.path ? join(pkgDir, record.path) : pkgDir;

    // Static security scan on the new version's content
    const scanResult = await scanStatic(newSkillDir);
    const warnings = scanResult.ok ? scanResult.value : [];

    if (await shouldSkipUpdate(warnings, options, record.name)) {
      return skipSkill(result, options, record.name);
    }

    // Replace the installed skill directory
    const installDir = currentSkillDir(record, options.projectRoot);
    const rmResult = await wrapShell(
      () => $`rm -rf ${installDir}`.quiet().then(() => undefined),
      `Failed to remove old skill directory '${record.name}'`,
    );
    if (!rmResult.ok) return rmResult;

    await mkdir(dirname(installDir), { recursive: true });

    const cpResult = await wrapShell(
      () => $`cp -r ${newSkillDir} ${installDir}`.quiet().then(() => undefined),
      `Failed to install updated skill '${record.name}'`,
      "Check disk space and permissions.",
    );
    if (!cpResult.ok) return cpResult;

    // Semantic scan on updated content
    if (await runUpdateSemanticScan(installDir, record.name, options)) {
      return skipSkill(result, options, record.name);
    }

    await refreshAgentSymlinks(record, options.projectRoot);

    // Re-verify trust for the new version
    const newTrust = await _resolveTrust({
      adapter: "npm",
      // biome-ignore lint/style/noNonNullAssertion: this branch only runs for npm skills (record.repo set)
      url: record.repo!,
      tap: record.tap,
      tarballPath: join(tmpDir, "_pkg.tgz"),
      npmPackageName: packageName,
      npmVersion: latestVersion,
      npmPublisher: record.trust?.publisher?.name,
    });

    // Update record in place
    patchRecord(installed, record, {
      ref: latestVersion,
      sha: null,
      updatedAt: new Date().toISOString(),
      trust: newTrust,
    });

    result.updated.push(record.name);
    options.onProgress?.(record.name, "updated");
    return ok(undefined);
  } finally {
    await removeTmpDir(tmpDir);
  }
}
