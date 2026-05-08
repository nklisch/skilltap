import { intro, log, outro, spinner } from "@clack/prompts";
import type {
  AgentAdapter,
  Config,
  EffectivePolicy,
  OrphanRecord,
  ScannedSkill,
  SemanticWarning,
  StaticWarning,
  TapEntry,
} from "@skilltap/core";
import {
  ensureBuiltinTap,
  findProjectRoot,
  formatOrphanReason,
  installMcpOnly,
  installSkill,
  isBuiltinTapCloned,
  loadManifest,
  manifestExists,
  parseMcpRef,
  recoverManifest,
  saveConfig,
  skillInstallDir,
  updateSkill,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { inferAdapter, sendEvent, telemetryBase } from "../telemetry";
import {
  agentError,
  agentSecurityBlock,
  agentSuccess,
  exitWithError,
} from "../ui/agent-out";
import { errorLine, successLine } from "../ui/format";
import {
  createInstallCallbacks,
  printCaptureConflict,
  printCaptureSummary,
} from "../ui/install-callbacks";
import { createStepLogger } from "../ui/install-steps";
import { componentSummary } from "../ui/plugin-format";
import { loadPolicyOrExit } from "../ui/policy";
import { confirmSaveDefault, selectAgents } from "../ui/prompts";
import {
  parseAlsoFlag,
  resolveAgentForAgentMode,
  resolveScope,
  resolveSemanticInteractive,
} from "../ui/resolve";

export default defineCommand({
  meta: {
    name: "install",
    description: "Install a skill from a URL, tap name, or local path",
  },
  args: {
    source: {
      type: "positional",
      description:
        "Git URL, github:owner/repo, tap skill name, mcp:<server>, or local path",
      required: true,
    },
    project: {
      type: "boolean",
      description: "Install to .agents/skills/ in current project",
      default: false,
    },
    global: {
      type: "boolean",
      description: "Install to ~/.agents/skills/",
      default: false,
    },
    also: {
      description: "Create symlink in agent-specific directory",
      valueHint: "agent",
    },
    ref: {
      description: "Branch or tag to install",
      valueHint: "ref",
    },
    "skip-scan": {
      type: "boolean",
      description: "Skip security scanning (not available in agent mode)",
      default: false,
    },
    yes: {
      type: "boolean",
      alias: "y",
      description: "Auto-accept prompts",
      default: false,
    },
    strict: {
      type: "boolean",
      description: "Abort on any security warning",
    },
    "no-strict": {
      type: "boolean",
      description: "Override config on_warn=fail for this invocation",
    },
    quiet: {
      type: "boolean",
      description:
        "Suppress install step details (overrides verbose = true in config)",
    },
    semantic: {
      type: "boolean",
      description: "Force semantic scan",
      default: false,
    },
    agent: {
      type: "boolean",
      description: "Run in non-interactive agent mode (also: SKILLTAP_AGENT=1)",
      default: false,
    },
  },
  async run({ args }) {
    const { config, policy } = await loadPolicyOrExit({
      strict: args.strict,
      noStrict: args["no-strict"],
      skipScan: args["skip-scan"],
      yes: args.yes,
      semantic: args.semantic,
      project: args.project,
      global: args.global,
      agent: args.agent,
    });

    // Ensure built-in tap is cloned before resolving tap names
    if (config.builtin_tap !== false) {
      const alreadyCloned = await isBuiltinTapCloned();
      if (!alreadyCloned) {
        await ensureBuiltinTap();
      }
    }

    const verbose = args.quiet ? false : config.verbose;
    const sources = args._ as string[];

    // Phase 35b: dispatch mcp:<source> to MCP-only install path. Mixing
    // mcp: and regular sources in one invocation is rejected.
    const hasMcp = sources.some((s) => s.startsWith("mcp:"));
    if (hasMcp) {
      if (!sources.every((s) => s.startsWith("mcp:"))) {
        errorLine(
          "Cannot mix mcp: and regular sources in one install. Run them separately.",
        );
        process.exit(1);
      }
      return runMcpInstall(sources, args, config, policy);
    }

    if (policy.agentMode) {
      return runAgentMode(sources, args, config, policy);
    }
    return runInteractiveMode(sources, args, config, policy, verbose);
  },
});

// ─── Manifest preflight (corruption recovery) ────────────────────────────────

/**
 * If the project's skilltap.toml exists but won't parse, decide what to do
 * BEFORE the install runs any heavy work (clone, scan, file placement).
 *
 * Without this preflight, install would proceed and silently swallow the
 * manifest update at the end (addSkillToManifest treats loadManifest failure
 * as a no-op so a "manifest hiccup doesn't roll back a successful install").
 * The result was a half-managed project: state.json updated, files placed,
 * skilltap.toml still corrupt, lockfile missing the new entry.
 *
 * Behavior:
 * - **Agent mode**: refuse and exit 1 with a pointer to `doctor --fix`. CI
 *   and scripts should never silently mutate user files.
 * - **Interactive mode**: auto-recover (backup to .bak, reset to empty),
 *   announce loudly, then proceed. The user gets their install AND a clean
 *   recovery path (their old content survives at skilltap.toml.bak).
 */
async function preflightManifestValidity(
  projectRoot: string | undefined,
  agentMode: boolean,
): Promise<void> {
  if (!projectRoot) return;
  if (!(await manifestExists(projectRoot))) return;

  const result = await loadManifest(projectRoot);
  if (result.ok) return;

  if (agentMode) {
    // exitWithError's agent-mode branch drops the hint, so fold the recovery
    // pointer into the message itself — the user/script needs to see both.
    exitWithError(
      true,
      `skilltap.toml is corrupt: ${result.error.message}\n` +
        "Run 'skilltap doctor --fix' to back up the corrupt manifest and reset to empty, then retry.",
    );
  }

  log.warn(`skilltap.toml is corrupt: ${result.error.message}`);
  log.info(
    "Backing up to skilltap.toml.bak and resetting to empty before install.",
  );
  await recoverManifest(projectRoot);
}

// ─── MCP-only install (Phase 35b) ─────────────────────────────────────────────

async function runMcpInstall(
  sources: string[],
  args: { also?: string },
  config: Config,
  policy: EffectivePolicy,
): Promise<void> {
  const scope = (policy.scope || "project") as "global" | "project";
  const projectRoot = scope === "project" ? await findProjectRoot() : undefined;
  const agents = parseAlsoFlag(args.also as string | undefined, config);
  const effectiveAgents = agents.length > 0 ? agents : ["claude-code"];

  for (const source of sources) {
    const ref = parseMcpRef(source);
    if (!ref) {
      errorLine(`Invalid mcp: source: ${source}`);
      process.exit(1);
    }

    const result = await installMcpOnly(source, {
      scope,
      projectRoot,
      agents: effectiveAgents,
      gitHost: config.default_git_host,
    });

    if (!result.ok) {
      errorLine(result.error.message, result.error.hint);
      process.exit(1);
    }

    const r = result.value;
    if (policy.agentMode) {
      agentSuccess(
        `Installed ${r.records.length} MCP server${r.records.length === 1 ? "" : "s"} from ${source} into ${r.agents.join(", ")}`,
      );
    } else {
      successLine(
        `Installed ${r.records.length} MCP server${r.records.length === 1 ? "" : "s"} from ${source} → ${r.agents.join(", ")}`,
      );
      for (const record of r.records) {
        successLine(`  • ${record.name}`);
      }
    }
  }
}

// ─── Agent Mode ───────────────────────────────────────────────────────────────

async function runAgentMode(
  sources: string[],
  args: { ref?: string; also?: string },
  config: Config,
  policy: EffectivePolicy,
): Promise<void> {
  const scope = policy.scope as "global" | "project";
  const projectRoot = scope === "project" ? await findProjectRoot() : undefined;

  // Refuse to proceed if the project's skilltap.toml is corrupt — agent mode
  // never silently mutates user files. (Interactive mode auto-recovers; see
  // preflightManifestValidity.)
  await preflightManifestValidity(projectRoot, true);

  let agent: AgentAdapter | undefined;
  if (policy.scanMode === "semantic") {
    agent = await resolveAgentForAgentMode(config);
  }

  const also = parseAlsoFlag(args.also, config);

  for (const source of sources) {
    const result = await installSkill(source, {
      scope,
      projectRoot,
      also,
      ref: args.ref,
      skipScan: false,
      gitHost: config.default_git_host,
      onWarnings: async (warnings: StaticWarning[]): Promise<boolean> => {
        agentSecurityBlock(warnings, []);
        process.exit(1);
        return false;
      },
      onSelectSkills: async (skills: ScannedSkill[]): Promise<string[]> =>
        skills.map((s) => s.name),
      onSelectTap: async (matches: TapEntry[]): Promise<TapEntry | null> =>
        matches[0] ?? null,
      onAlreadyInstalled: async () => "update" as const,
      agent,
      semantic: policy.scanMode === "semantic",
      threshold: config.security.threshold,
      onSemanticWarnings: async (
        warnings: SemanticWarning[],
      ): Promise<boolean> => {
        agentSecurityBlock([], warnings);
        process.exit(1);
        return false;
      },
      async onOrphansFound(orphans: OrphanRecord[]) {
        for (const o of orphans) {
          process.stdout.write(
            `warning: Stale record "${o.record.name}" — ${formatOrphanReason(o.reason)}. Auto-removing.\n`,
          );
        }
        return orphans.map((o) => o.record.name);
      },
      onPluginDetected: async () => "plugin" as const,
      onPluginWarnings: async (warnings: StaticWarning[]): Promise<boolean> => {
        agentSecurityBlock(warnings, []);
        process.exit(1);
        return false;
      },
      // Agent mode: same-source captures auto-confirm; cross-source conflicts
      // hard-abort (the UserError carries the resolution hint).
      onPluginCaptureConflict: async () => "abort" as const,
      onPluginCaptureConfirm: async () => true,
    });

    if (!result.ok) {
      sendEvent(config, "install", {
        ...telemetryBase(true),
        adapter: inferAdapter(source),
        success: false,
        error_category: result.error.constructor.name,
        skill_count: 0,
        scan_mode: policy.scanMode,
        scope,
      });
      agentError(result.error.message);
      process.exit(1);
    }

    sendEvent(config, "install", {
      ...telemetryBase(true),
      adapter: inferAdapter(source),
      success: true,
      skill_count: result.value.records.length,
      scan_mode: policy.scanMode,
      scope,
    });

    for (const record of result.value.records) {
      const installDir = skillInstallDir(record.name, scope, projectRoot);
      agentSuccess(record.name, installDir, record.ref, record.trust);
    }

    if (result.value.pluginRecord) {
      const pr = result.value.pluginRecord;
      const summary = componentSummary(pr);
      process.stdout.write(`OK: Installed plugin ${pr.name} (${summary})\n`);
      const cap = result.value.captured;
      if (cap && cap.skills.length + cap.mcpServers.length > 0) {
        process.stdout.write(
          `Captured ${cap.skills.length} standalone skill(s), ${cap.mcpServers.length} MCP server(s) into "${pr.name}".\n`,
        );
        const forced = cap.forcedCrossSource;
        if (forced.skills.length + forced.mcpServers.length > 0) {
          const names = [...forced.skills, ...forced.mcpServers].join(", ");
          process.stdout.write(
            `  ⚠ Force-captured (cross-source override): ${names}\n`,
          );
        }
      }
    }

    for (const name of result.value.updates) {
      const updateResult = await updateSkill({
        name,
        yes: true,
        projectRoot,
        agent,
        semantic: policy.scanMode === "semantic",
        threshold: config.security.threshold,
        onSemanticWarnings: (warnings) => {
          agentSecurityBlock([], warnings);
          process.exit(1);
        },
      });
      if (!updateResult.ok) {
        agentError(updateResult.error.message);
        process.exit(1);
      }
      const { updated, upToDate } = updateResult.value;
      if (updated.includes(name)) {
        process.stdout.write(`OK: Updated ${name}\n`);
      } else if (upToDate.includes(name)) {
        process.stdout.write(`OK: ${name} is already up to date.\n`);
      }
    }
  }
}

// ─── Interactive Mode ─────────────────────────────────────────────────────────

async function runInteractiveMode(
  sources: string[],
  args: {
    ref?: string;
    also?: string;
    semantic: boolean;
  },
  config: Config,
  policy: EffectivePolicy,
  verbose: boolean,
): Promise<void> {
  const { onWarn, skipScan } = policy;
  let also = parseAlsoFlag(args.also, config);

  const { runSemantic, agent } = await resolveSemanticInteractive(
    policy,
    args,
    config,
  );

  intro("skilltap");

  // Scope: policy already resolved from flags + config. Only prompt if still "".
  let scope: "global" | "project";
  let projectRoot: string | undefined;
  if (policy.scope) {
    scope = policy.scope as "global" | "project";
    if (scope === "project") projectRoot = await findProjectRoot();
  } else {
    const resolved = await resolveScope({}, undefined);
    scope = resolved.scope;
    projectRoot = resolved.projectRoot;
  }

  // Auto-recover a corrupt skilltap.toml before doing any install work, so
  // we don't end up with state.json updated + manifest stale. The recovery
  // backs up the corrupt file to skilltap.toml.bak and resets to empty.
  await preflightManifestValidity(projectRoot, false);

  // Prompt for agent symlinks unless --also was explicitly passed, --yes is set,
  // or the user already has a saved default in config
  if (!args.also && !policy.yes && !config.defaults.also.length) {
    const selected = await selectAgents(also);
    also = selected;

    // Offer to save if selection differs from config default
    const configAlso = config.defaults.also;
    const differs =
      also.length !== configAlso.length ||
      also.some((a) => !configAlso.includes(a));
    if (differs) {
      const save = await confirmSaveDefault("Save agent selection as default?");
      if (save) {
        config.defaults.also = also;
        await saveConfig(config);
      }
    }
  }

  const errors: { source: string; message: string; hint?: string }[] = [];

  for (const source of sources) {
    const s = spinner();
    s.start(`Fetching ${source}...`);

    const steps = createStepLogger(verbose);
    const { callbacks, logScanResults } = createInstallCallbacks({
      spinner: s,
      onWarn,
      skipScan,
      agent,
      yes: policy.yes,
      source,
      steps,
    });

    // Closure shared between onPluginCaptureConflict and onPluginCaptureConfirm
    // to track which names were force-overridden.
    const forcedCaptureNames = new Set<string>();

    const result = await installSkill(source, {
      scope,
      projectRoot,
      also,
      ref: args.ref,
      skipScan,
      gitHost: config.default_git_host,
      agent,
      semantic: runSemantic,
      threshold: config.security.threshold,
      ...callbacks,
      async onOrphansFound(orphans: OrphanRecord[]) {
        if (orphans.length === 0) return [];
        if (policy.yes) {
          for (const o of orphans) {
            log.warn(
              `Stale record "${o.record.name}" (${formatOrphanReason(o.reason)}). Auto-removing.`,
            );
          }
          return orphans.map((o) => o.record.name);
        }
        log.warn(`Found ${orphans.length} stale record(s):`);
        for (const o of orphans) {
          log.warn(`  ${o.record.name}: ${formatOrphanReason(o.reason)}`);
        }
        const { confirm: confirmPrompt, isCancel: isCancelPrompt } =
          await import("@clack/prompts");
        const shouldClean = await confirmPrompt({
          message: "Remove stale records? (directories are already gone)",
          initialValue: true,
        });
        if (isCancelPrompt(shouldClean)) process.exit(130);
        if (!shouldClean) return [];
        return orphans.map((o) => o.record.name);
      },
      async onPluginCaptureConflict(crossSource) {
        s.stop();
        printCaptureConflict(crossSource, source);
        const { isCancel: isCancelPrompt } = await import("@clack/prompts");
        const { footerSelect: footerSel } = await import("../ui/footer");
        const decision = await footerSel<"abort" | "force">({
          message:
            "Cross-source capture conflict — what do you want to do?",
          initialValue: "abort",
          options: [
            {
              value: "abort" as const,
              label: "Abort the install (recommended)",
            },
            {
              value: "force" as const,
              label:
                "Force capture (override and replace standalones from a different source)",
            },
          ],
        });
        if (isCancelPrompt(decision)) {
          s.start(`Fetching ${source}...`);
          return "abort";
        }
        const resolved = decision as "abort" | "force";
        if (resolved === "force") {
          for (const c of crossSource.skills) {
            forcedCaptureNames.add(c.standalone.name);
          }
          for (const c of crossSource.mcpServers) {
            forcedCaptureNames.add(c.serverName);
          }
        }
        s.start(`Fetching ${source}...`);
        return resolved;
      },
      async onPluginCaptureConfirm(bucket) {
        if (policy.yes) return true;
        s.stop();
        printCaptureSummary(bucket, source, forcedCaptureNames);
        const { isCancel: isCancelPrompt } = await import("@clack/prompts");
        const { footerConfirm: footerConf } = await import("../ui/footer");
        const proceed = await footerConf({
          message: `Capture these components into the plugin?`,
          initialValue: true,
        });
        if (isCancelPrompt(proceed) || proceed === false) {
          s.start(`Fetching ${source}...`);
          return false;
        }
        s.start(`Fetching ${source}...`);
        return true;
      },
    });

    if (!result.ok) {
      s.stop();
      sendEvent(config, "install", {
        ...telemetryBase(false),
        adapter: inferAdapter(source),
        success: false,
        error_category: result.error.constructor.name,
        skill_count: 0,
        scan_mode: policy.scanMode,
        scope,
      });
      errors.push({
        source,
        message: result.error.message,
        hint: result.error.hint,
      });
      continue;
    }

    s.stop();
    logScanResults();

    sendEvent(config, "install", {
      ...telemetryBase(false),
      adapter: inferAdapter(source),
      success: true,
      skill_count: result.value.records.length,
      scan_mode: policy.scanMode,
      scope,
    });

    for (const record of result.value.records) {
      const installDir = skillInstallDir(record.name, scope, projectRoot);
      successLine(`Installed ${record.name} → ${installDir}`);
    }

    if (result.value.pluginRecord) {
      const pr = result.value.pluginRecord;
      const summary = componentSummary(pr);
      successLine(`Installed plugin ${pr.name} (${summary})`);
      const cap = result.value.captured;
      if (cap && cap.skills.length + cap.mcpServers.length > 0) {
        successLine(
          `Captured ${cap.skills.length} standalone skill(s), ${cap.mcpServers.length} MCP server(s) into "${pr.name}".`,
        );
        const forced = cap.forcedCrossSource;
        if (forced.skills.length + forced.mcpServers.length > 0) {
          const names = [...forced.skills, ...forced.mcpServers].join(", ");
          log.warn(`  ⚠ Force-captured (cross-source override): ${names}`);
        }
      }
    }

    // Run updates for skills that were already installed
    for (const name of result.value.updates) {
      const updateResult = await updateSkill({
        name,
        yes: policy.yes,
        projectRoot,
        agent,
        semantic: runSemantic,
        threshold: config.security.threshold,
      });
      if (!updateResult.ok) {
        errorLine(updateResult.error.message, updateResult.error.hint);
      } else {
        const { updated, upToDate } = updateResult.value;
        if (updated.includes(name)) successLine(`Updated ${name}`);
        else if (upToDate.includes(name))
          log.info(`${name} is already up to date.`);
      }
    }
  }

  if (errors.length > 0) {
    for (const { source, message, hint } of errors) {
      errorLine(`${source}: ${message}`, hint);
    }
    outro("Finished with errors.");
    process.exit(1);
  }

  outro("Complete!");
}
