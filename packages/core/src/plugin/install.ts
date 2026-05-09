import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { $ } from "bun";
import { getConfigDir } from "../config";
import { addPluginToManifest } from "../manifest/update";
import { agentDefPath, skillInstallDir } from "../paths";
import type {
  PluginAgentComponent,
  PluginManifest,
  PluginMcpComponent,
} from "../schemas/plugin";
import type { PluginRecord, StoredMcpComponent } from "../schemas/plugins";
import type { StaticWarning } from "../security/static";
import { scanStatic } from "../security/static";
import { wrapShell } from "../shell";
import { loadState } from "../state/load";
import { createAgentSymlinks } from "../symlink";
import { DEFAULT_AGENT_ID } from "../symlink";
import { err, ok, type Result, type ScanError, UserError } from "../types";
import {
  applyCapture,
  buildCrossSourceHint,
  type CaptureBucket,
  detectCaptureMatches,
  mergeBuckets,
} from "./capture";
import { injectMcpServers } from "./mcp-inject";
import {
  addPlugin,
  loadPlugins,
  manifestToRecord,
  mcpServerToStored,
  savePlugins,
} from "./state";

export type PluginInstallOptions = {
  scope: "global" | "project";
  projectRoot?: string;
  also?: string[];
  skipScan?: boolean;
  /** Called when static security warnings found. Return true to proceed. */
  onWarnings?: (
    warnings: StaticWarning[],
    pluginName: string,
  ) => Promise<boolean>;
  /** Called before placement for confirmation. Return false to cancel. */
  onConfirm?: (manifest: PluginManifest) => Promise<boolean>;
  /**
   * Called when the plugin's components collide with already-installed
   * standalones from the SAME canonical source. Return true to capture, false
   * to abort. Omitted with non-empty same-source matches → auto-capture
   * (matches the existing pattern where a missing confirm callback
   * auto-proceeds).
   */
  onCaptureConfirm?: (sameSource: CaptureBucket) => Promise<boolean>;
  /**
   * Called when the plugin's components collide with already-installed
   * standalones from a DIFFERENT canonical source — or with no recorded
   * source at all. Returns:
   *   "abort" — fail the install with a UserError.
   *   "force" — treat conflicts as captures (user override). Conflicts merge
   *             into the same-source bucket and flow through the normal
   *             apply path; if `onCaptureConfirm` is provided it sees the
   *             merged set.
   *   "skip"  — leave the cross-source standalones alone and continue
   *             installing the plugin side-by-side. Same-source captures
   *             still proceed via `onCaptureConfirm`. Used by the CLI's
   *             `--no-capture` flag.
   * Omitted with non-empty cross-source conflicts → install fails with
   * a UserError. (Auto-confirm modes opt in by passing `() => "abort"`
   * explicitly; interactive modes that want force-override pass a real
   * callback.)
   */
  onCaptureConflict?: (
    crossSource: CaptureBucket,
  ) => Promise<"abort" | "force" | "skip">;
  /**
   * Skip the entire capture phase. Same-source and cross-source standalones
   * are left in place and the plugin installs side-by-side. Used by the CLI's
   * `--no-capture` flag. When true, neither `onCaptureConfirm` nor
   * `onCaptureConflict` is invoked.
   */
  skipCapture?: boolean;
  /** Repo URL for recording */
  repo: string | null;
  /** Git ref */
  ref: string | null;
  /** Git SHA */
  sha: string | null;
  /** Tap name if installed from a tap */
  tap: string | null;
};

export type PluginInstallResult = {
  record: PluginRecord;
  warnings: StaticWarning[];
  /** List of agents where MCP was injected */
  mcpAgents: string[];
  /** Number of agent definitions placed */
  agentDefsPlaced: number;
  /** Components transferred from standalone state to this plugin. */
  captured: {
    skills: string[];
    mcpServers: string[];
    /**
     * Subset of the above whose ownership transferred via cross-source force
     * override. Empty unless the user invoked `onCaptureConflict → "force"`.
     */
    forcedCrossSource: { skills: string[]; mcpServers: string[] };
  };
};

/**
 * Install a plugin from a pre-cloned directory.
 *
 * 1. Security scan all plugin content
 * 2. Place skills in .agents/skills/ with agent symlinks
 * 3. Inject MCP server configs into target agent config files
 * 4. Place agent definitions in .claude/agents/
 * 5. Record plugin in state.json
 */
export async function installPlugin(
  contentDir: string,
  manifest: PluginManifest,
  options: PluginInstallOptions,
): Promise<Result<PluginInstallResult, UserError | ScanError>> {
  const { scope, projectRoot, also = [], skipScan } = options;

  // 1. Security scan
  let warnings: StaticWarning[] = [];
  if (!skipScan) {
    const scanResult = await scanStatic(contentDir);
    if (!scanResult.ok) return scanResult;
    warnings = scanResult.value;

    if (warnings.length > 0) {
      if (!options.onWarnings) {
        return err(
          new UserError(
            `Security warnings found in plugin "${manifest.name}". Aborting.`,
            "Use skipScan to bypass (not recommended).",
          ),
        );
      }
      const proceed = await options.onWarnings(warnings, manifest.name);
      if (!proceed) {
        return err(
          new UserError(
            `Install of plugin "${manifest.name}" cancelled due to security warnings.`,
          ),
        );
      }
    }
  }

  // 1.5. Capture detection — before any on-disk changes, check whether the
  // plugin's components collide with already-installed standalones, partition
  // by source provenance, and run the capture/conflict callbacks.
  let capturedSkills: string[] = [];
  let capturedMcpServers: string[] = [];
  let forcedBucket: CaptureBucket = { skills: [], mcpServers: [] };

  if (!options.skipCapture) {
    const stateForCapture = await loadState(
      scope === "project" ? projectRoot : undefined,
    );
    if (!stateForCapture.ok) return stateForCapture;
    const matches = detectCaptureMatches(
      stateForCapture.value,
      manifest,
      options.repo,
    );

    let toCapture: CaptureBucket = matches.sameSource;

    // Cross-source conflicts evaluated FIRST. A force decision merges them
    // into the capture set.
    if (matches.crossSourceTotal > 0) {
      if (!options.onCaptureConflict) {
        return err(
          new UserError(
            `Plugin "${manifest.name}" would replace ${matches.crossSourceTotal} standalone component(s) installed from a different source.`,
            buildCrossSourceHint(matches.crossSource, options.repo),
          ),
        );
      }
      const decision = await options.onCaptureConflict(matches.crossSource);
      if (decision === "abort") {
        return err(
          new UserError(
            `Install of plugin "${manifest.name}" cancelled — cross-source capture conflict.`,
          ),
        );
      }
      if (decision === "force") {
        forcedBucket = matches.crossSource;
        toCapture = mergeBuckets(matches.sameSource, forcedBucket);
      }
      // decision === "skip" → leave cross-source standalones alone, proceed
      // installing the plugin. toCapture stays as same-source only.
    }

    if (toCapture.skills.length + toCapture.mcpServers.length > 0) {
      if (options.onCaptureConfirm) {
        const proceed = await options.onCaptureConfirm(toCapture);
        if (!proceed) {
          return err(
            new UserError(`Install of plugin "${manifest.name}" cancelled.`),
          );
        }
      }
      const applied = await applyCapture(toCapture, {
        scope,
        projectRoot,
        pluginName: manifest.name,
      });
      if (!applied.ok) return applied;
      capturedSkills = applied.value.capturedSkills;
      capturedMcpServers = applied.value.capturedMcpServers;
    }
  }

  // 2. Place skills
  const skillComponents = manifest.components.filter((c) => c.type === "skill");
  for (const component of skillComponents) {
    const src = join(contentDir, component.path);
    const dest = skillInstallDir(component.name, scope, projectRoot);

    const mkdirResult = await wrapShell(
      () => mkdir(dirname(dest), { recursive: true }).then(() => undefined),
      `Failed to create skill directory for "${component.name}"`,
    );
    if (!mkdirResult.ok) return mkdirResult;

    const cpResult = await wrapShell(
      () => $`cp -a ${src} ${dest}`.quiet().then(() => undefined),
      `Failed to copy skill "${component.name}"`,
      "Check that the skill path exists in the plugin.",
    );
    if (!cpResult.ok) return cpResult;

    if (also.length > 0) {
      const symlinkResult = await createAgentSymlinks(
        component.name,
        dest,
        also,
        scope,
        projectRoot,
      );
      if (!symlinkResult.ok) return symlinkResult;
    }
  }

  // 3. Inject MCP servers
  const mcpComponents = manifest.components.filter(
    (c): c is PluginMcpComponent => c.type === "mcp",
  );
  const storedMcpComponents: StoredMcpComponent[] = [];
  for (const component of mcpComponents) {
    storedMcpComponents.push(mcpServerToStored(component.server));
  }

  let mcpAgents: string[] = [];
  if (storedMcpComponents.length > 0 && also.length > 0) {
    const vars = {
      pluginRoot: contentDir,
      pluginData: join(getConfigDir(), "plugin-data", manifest.name),
    };
    const injectResult = await injectMcpServers({
      pluginName: manifest.name,
      servers: storedMcpComponents,
      agents: also,
      scope,
      projectRoot,
      vars,
    });
    if (!injectResult.ok) return injectResult;
    mcpAgents = injectResult.value;
  }

  // 4. Place agent definitions
  const agentComponents = manifest.components.filter(
    (c): c is PluginAgentComponent => c.type === "agent",
  );
  let agentDefsPlaced = 0;
  for (const component of agentComponents) {
    const src = join(contentDir, component.path);
    const dest = agentDefPath(
      component.name,
      DEFAULT_AGENT_ID,
      scope,
      projectRoot,
    );
    if (!dest) continue;

    try {
      await mkdir(dirname(dest), { recursive: true });
      await Bun.write(dest, Bun.file(src));
      agentDefsPlaced++;
    } catch (e) {
      return err(
        new UserError(
          `Failed to place agent definition "${component.name}": ${e}`,
        ),
      );
    }
  }

  // 5. Record in state.json
  const record = manifestToRecord(manifest, {
    repo: options.repo,
    ref: options.ref,
    sha: options.sha,
    scope,
    also,
    tap: options.tap,
  });

  const loadResult = await loadPlugins(projectRoot);
  if (!loadResult.ok) return loadResult;
  const newState = addPlugin(loadResult.value, record);
  const saveResult = await savePlugins(newState, projectRoot);
  if (!saveResult.ok) return saveResult;

  // v2 manifest update — no-op without skilltap.toml.
  // Only fires for project-scope installs in a project root that has a
  // manifest. Failures are non-fatal — the plugin is already installed.
  if (options.scope === "project" && projectRoot && record.repo) {
    await addPluginToManifest(projectRoot, {
      source: record.repo,
      ref: record.ref,
      sha: record.sha,
    }).catch(() => {
      // non-fatal
    });
  }

  return ok({
    record,
    warnings,
    mcpAgents,
    agentDefsPlaced,
    captured: {
      skills: capturedSkills,
      mcpServers: capturedMcpServers,
      forcedCrossSource: {
        skills: forcedBucket.skills.map((c) => c.standalone.name),
        mcpServers: forcedBucket.mcpServers.map((c) => c.serverName),
      },
    },
  });
}
