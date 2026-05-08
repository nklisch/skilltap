import { rename } from "node:fs/promises";
import { join } from "node:path";
import { parse, stringify } from "smol-toml";
import { ensureDirs, getConfigDir } from "../config";
import { parseWithResult } from "../schemas/index";
import { type InstalledJson, InstalledJsonSchema } from "../schemas/installed";
import { type PluginsJson, PluginsJsonSchema } from "../schemas/plugins";
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
  if (configResult && globalMarkers.configToml && globalMarkers.configHasV1Keys) {
    const newText = stringify(configResult.v2 as Record<string, unknown>);
    const tmpPath = join(getConfigDir(), "config.toml.v2.tmp");
    await Bun.write(tmpPath, newText);

    const bakPath = `${globalMarkers.configToml}.v1.bak`;
    await rename(globalMarkers.configToml, bakPath);
    renamed.push({ from: globalMarkers.configToml, to: bakPath });

    await rename(tmpPath, globalMarkers.configToml);
    written.push(globalMarkers.configToml);
  }

  return ok({
    alreadyMigrated: false,
    scopes,
    changes: { written, renamed },
    warnings,
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
  const saveResult = await saveState(newState, projectRoot);
  if (!saveResult.ok) return saveResult;
  written.push(
    projectRoot
      ? join(projectRoot, ".agents", "state.json")
      : join(getConfigDir(), "state.json"),
  );

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
