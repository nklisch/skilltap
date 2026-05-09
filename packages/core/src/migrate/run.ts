import { rename } from "node:fs/promises";
import { join } from "node:path";
import { parse, stringify } from "smol-toml";
import { ensureDirs, getConfigDir } from "../config";
import { debug } from "../debug";
import { type DoctorResult, runDoctor } from "../doctor";
import {
  addMcpToLockfile,
  addMcpToManifest,
  addPluginToManifest,
  addSkillToManifest,
  manifestExists,
} from "../manifest";
import { parseWithResult } from "../schemas/index";
import {
  type LegacyInstalledJson as InstalledJson,
  LegacyInstalledJsonSchema as InstalledJsonSchema,
  type LegacyPluginsJson as PluginsJson,
  LegacyPluginsJsonSchema as PluginsJsonSchema,
} from "./legacy-schemas";
import { loadState } from "../state/load";
import { migrateV1State } from "../state/migrate-v1";
import { saveState } from "../state/save";
import { err, ok, type Result, UserError } from "../types";
import { type ConfigMigrationResult, migrateV1Config } from "./config-v1";
import {
  detectV1StateGlobal,
  detectV1StateProject,
  hasAnyV1Markers,
  type V1StateMarkers,
} from "./detect";

export interface MigrationFileChange {
  written: string[];
  renamed: { from: string; to: string }[];
}

export interface MigrationReport {
  alreadyMigrated: boolean;
  scopes: ("global" | "project")[];
  changes: MigrationFileChange;
  warnings: string[];
  /** Doctor result from post-migrate verification. Only present on non-no-op migrations. */
  doctorReport?: DoctorResult;
}

export interface MigrateOptions {
  projectRoot?: string;
}

export async function runMigrate(
  options: MigrateOptions = {},
): Promise<Result<MigrationReport, UserError>> {
  const dirsResult = await ensureDirs();
  if (!dirsResult.ok) return dirsResult;

  const globalMarkers = await detectV1StateGlobal();
  const projectMarkers = options.projectRoot
    ? await detectV1StateProject(options.projectRoot)
    : null;

  const anyGlobal = hasAnyV1Markers(globalMarkers);
  const anyProject = projectMarkers !== null && hasAnyV1Markers(projectMarkers);

  if (!anyGlobal && !anyProject) {
    return ok({
      alreadyMigrated: true,
      scopes: [],
      changes: { written: [], renamed: [] },
      warnings: [],
    });
  }

  const warnings: string[] = [];
  const written: string[] = [];
  const renamed: { from: string; to: string }[] = [];
  const scopes: ("global" | "project")[] = [];

  // ── Translate global config first; refuse on HTTP taps before any writes ──
  let configResult: ConfigMigrationResult | null = null;
  if (globalMarkers.configToml) {
    const text = await Bun.file(globalMarkers.configToml).text();
    let raw: unknown;
    try {
      raw = parse(text);
    } catch (e) {
      return err(
        new UserError(`Invalid TOML in ${globalMarkers.configToml}: ${e}`),
      );
    }
    const result = migrateV1Config(raw);
    if (!result.ok) return result;
    configResult = result.value;

    if (configResult.httpTapsRejected.length > 0) {
      const list = configResult.httpTapsRejected
        .map((t) => `  - ${t.name} (${t.url})`)
        .join("\n");
      return err(
        new UserError(
          `Migration aborted: HTTP taps are not supported in v2.0. Convert these to git or remove them, then re-run:\n${list}`,
          "edit ~/.config/skilltap/config.toml: remove HTTP [[taps]] entries or replace url= with a git URL.",
        ),
      );
    }
    warnings.push(...configResult.warnings);
  }

  // ── Migrate global state (installed.json + plugins.json → state.json) ────
  if (anyGlobal) {
    scopes.push("global");
    const stateResult = await migrateScopeState(globalMarkers);
    if (!stateResult.ok) return stateResult;
    written.push(...stateResult.value.written);
    renamed.push(...stateResult.value.renamed);
  }

  // ── Migrate project state (if a project root was provided) ───────────────
  if (anyProject && projectMarkers !== null) {
    scopes.push("project");
    const stateResult = await migrateScopeState(
      projectMarkers,
      options.projectRoot,
    );
    if (!stateResult.ok) return stateResult;
    written.push(...stateResult.value.written);
    renamed.push(...stateResult.value.renamed);
  }

  // ── Write migrated config last (after state is safely on disk) ───────────
  // Only overwrite config.toml if it actually contained v1 keys that needed
  // migration. If the config is already in the current format (no v1 keys),
  // leave it untouched to avoid replacing it with the v2-schema format.
  if (
    configResult &&
    globalMarkers.configToml &&
    globalMarkers.configHasV1Keys
  ) {
    const newText = stringify(configResult.migrated as Record<string, unknown>);
    const tmpPath = join(getConfigDir(), "config.toml.v2.tmp");
    await Bun.write(tmpPath, newText);

    const bakPath = `${globalMarkers.configToml}.v1.bak`;
    await rename(globalMarkers.configToml, bakPath);
    renamed.push({ from: globalMarkers.configToml, to: bakPath });

    await rename(tmpPath, globalMarkers.configToml);
    written.push(globalMarkers.configToml);
  }

  // ── Manifest verification (skilltap.toml parse check) ───────────────────
  if (options.projectRoot) {
    const manifestPath = join(options.projectRoot, "skilltap.toml");
    const manifestFile = Bun.file(manifestPath);
    if (await manifestFile.exists()) {
      try {
        const text = await manifestFile.text();
        parse(text);
      } catch (e) {
        warnings.push(
          `skilltap.toml at ${manifestPath} did not parse cleanly: ${e}`,
        );
      }
    }
  }

  // ── Doctor post-migrate verification ─────────────────────────────────────
  const doctorReport = await runDoctor({ projectRoot: options.projectRoot });

  return ok({
    alreadyMigrated: false,
    scopes,
    changes: { written, renamed },
    warnings,
    doctorReport,
  });
}

async function migrateScopeState(
  markers: V1StateMarkers,
  projectRoot?: string,
): Promise<Result<MigrationFileChange, UserError>> {
  const written: string[] = [];
  const renamed: { from: string; to: string }[] = [];

  let installed: InstalledJson = { version: 1, skills: [] };
  if (markers.installedJson) {
    let raw: unknown;
    try {
      raw = await Bun.file(markers.installedJson).json();
    } catch (e) {
      return err(
        new UserError(`Invalid JSON in ${markers.installedJson}: ${e}`),
      );
    }
    const parsed = parseWithResult(
      InstalledJsonSchema,
      raw,
      markers.installedJson,
    );
    if (!parsed.ok) return parsed;
    installed = parsed.value;
  }

  let plugins: PluginsJson = { version: 1, plugins: [] };
  if (markers.pluginsJson) {
    let raw: unknown;
    try {
      raw = await Bun.file(markers.pluginsJson).json();
    } catch (e) {
      return err(new UserError(`Invalid JSON in ${markers.pluginsJson}: ${e}`));
    }
    const parsed = parseWithResult(PluginsJsonSchema, raw, markers.pluginsJson);
    if (!parsed.ok) return parsed;
    plugins = parsed.value;
  }

  const newState = migrateV1State(installed, plugins);

  // Preserve any pre-existing state.mcpServers — those entries come from a
  // partial earlier migration or a restored backup and are not represented in
  // legacy installed.json/plugins.json. The legacy formats have no MCP
  // tracking, so this is purely additive.
  const existingState = await loadState(projectRoot).catch(() => null);
  const preservedMcps = existingState?.ok ? existingState.value.mcpServers : [];

  const saveResult = await saveState(
    { ...newState, mcpServers: preservedMcps },
    projectRoot,
  );
  if (!saveResult.ok) return saveResult;
  written.push(
    projectRoot
      ? join(projectRoot, ".agents", "state.json")
      : join(getConfigDir(), "state.json"),
  );

  // Lifecycle drift fix (Unit 3.15): seed the project manifest+lockfile from
  // the migrated state so subsequent `sync` runs see the same view. No-op
  // without skilltap.toml or for global migrations. Best-effort.
  if (projectRoot && (await manifestExists(projectRoot))) {
    for (const skill of newState.skills) {
      if (skill.scope !== "project") continue;
      if (!skill.repo) continue;
      await addSkillToManifest(projectRoot, {
        source: skill.repo,
        ref: skill.ref,
        sha: skill.sha,
      }).catch((e) =>
        debug("migrate: addSkillToManifest failed", {
          name: skill.name,
          error: String(e),
        }),
      );
    }
    for (const plugin of newState.plugins) {
      if (plugin.scope !== "project") continue;
      if (!plugin.repo) continue;
      await addPluginToManifest(projectRoot, {
        source: plugin.repo,
        ref: plugin.ref ?? null,
        sha: plugin.sha ?? null,
      }).catch((e) =>
        debug("migrate: addPluginToManifest failed", {
          name: plugin.name,
          error: String(e),
        }),
      );
    }
    for (const mcp of preservedMcps) {
      // Only entries with a `source` carry meaningful manifest semantics.
      if (!mcp.source) continue;
      await addMcpToManifest(projectRoot, {
        name: mcp.name,
        source: mcp.source,
      }).catch((e) =>
        debug("migrate: addMcpToManifest failed", {
          name: mcp.name,
          error: String(e),
        }),
      );
      // Lockfile sha may be unknown for legacy MCP records — seed only when
      // present to avoid producing an invalid lockfile entry.
      const sha = (mcp as { sha?: string }).sha;
      if (sha) {
        await addMcpToLockfile(projectRoot, {
          name: mcp.name,
          source: mcp.source,
          ref: "main",
          sha,
        }).catch((e) =>
          debug("migrate: addMcpToLockfile failed", {
            name: mcp.name,
            error: String(e),
          }),
        );
      }
    }
  }

  // Rename v1 files to .v1.bak (after state.json is safely written)
  if (markers.installedJson) {
    const bak = `${markers.installedJson}.v1.bak`;
    await rename(markers.installedJson, bak);
    renamed.push({ from: markers.installedJson, to: bak });
  }
  if (markers.pluginsJson) {
    const bak = `${markers.pluginsJson}.v1.bak`;
    await rename(markers.pluginsJson, bak);
    renamed.push({ from: markers.pluginsJson, to: bak });
  }

  return ok({ written, renamed });
}
