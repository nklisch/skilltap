import { intro, log } from "@clack/prompts";
import type {
  AgentAdapter,
  CaptureBucket,
  Config,
  EffectivePolicy,
  InstallOptions,
  OrphanRecord,
} from "@skilltap/core";
import {
  ensureBuiltinTap,
  findProjectRoot,
  formatOrphanReason,
  isBuiltinTapCloned,
  loadManifest,
  manifestExists,
  recoverManifest,
  saveConfig,
} from "@skilltap/core";
import type { Output, Progress } from "../../output";
import { createOutput } from "../../output";
import {
  createInstallCallbacks,
  printCaptureConflict,
  printCaptureSummary,
} from "../../ui/install-callbacks";
import { createStepLogger } from "../../ui/install-steps";
import { loadPolicyOrExit } from "../../ui/policy";
import { confirmSaveDefault, selectAgents } from "../../ui/prompts";
import {
  parseAlsoFlag,
  resolveScope,
  resolveSemanticInteractive,
} from "../../ui/resolve";

export type SharedInstallArgs = {
  source: string;
  project: boolean;
  global: boolean;
  also?: string;
  ref?: string;
  "skip-scan": boolean;
  yes: boolean;
  strict?: boolean;
  "no-strict"?: boolean;
  quiet?: boolean;
  semantic: boolean;
  json?: boolean;
};

export type InstallContext = {
  out: Output;
  config: Config;
  policy: EffectivePolicy;
  scope: "global" | "project";
  projectRoot: string | undefined;
  also: string[];
  runSemantic: boolean;
  agent: AgentAdapter | undefined;
  verbose: boolean;
};

export async function setupInstallContext(
  args: SharedInstallArgs,
): Promise<InstallContext> {
  const out = createOutput({
    json: args.json ?? false,
    quiet: args.quiet ?? false,
  });
  const { config, policy } = await loadPolicyOrExit({
    strict: args.strict,
    noStrict: args["no-strict"],
    skipScan: args["skip-scan"],
    yes: args.yes,
    semantic: args.semantic,
    project: args.project,
    global: args.global,
  });

  if (config.builtin_tap !== false) {
    const alreadyCloned = await isBuiltinTapCloned();
    if (!alreadyCloned) {
      await ensureBuiltinTap();
    }
  }

  const verbose = args.quiet ? false : config.verbose;

  const { runSemantic, agent } = await resolveSemanticInteractive(
    policy,
    args,
    config,
  );

  intro("skilltap");

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

  await preflightManifestValidity(projectRoot);

  let also = parseAlsoFlag(args.also, config);
  if (!args.also && !policy.yes && !config.defaults.also.length) {
    const selected = await selectAgents(also);
    also = selected;

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

  return {
    out,
    config,
    policy,
    scope,
    projectRoot,
    also,
    runSemantic,
    agent,
    verbose,
  };
}

async function preflightManifestValidity(
  projectRoot: string | undefined,
): Promise<void> {
  if (!projectRoot) return;
  if (!(await manifestExists(projectRoot))) return;

  const result = await loadManifest(projectRoot);
  if (result.ok) return;

  log.warn(`skilltap.toml is corrupt: ${result.error.message}`);
  log.info(
    "Backing up to skilltap.toml.bak and resetting to empty before install.",
  );
  await recoverManifest(projectRoot);
}

/**
 * Build all install callbacks for a single source install.
 * Returns the callback bag to spread into installSkill() options,
 * plus the progress handle and a logScanResults function.
 */
export function buildSourceCallbacks(
  ctx: InstallContext,
  source: string,
): {
  progress: Progress;
  logScanResults: () => void;
  installOptions: Pick<
    InstallOptions,
    | "onWarnings"
    | "onSelectSkills"
    | "onSelectTap"
    | "onAlreadyInstalled"
    | "onConfirmInstall"
    | "onDeepScan"
    | "onPluginDetected"
    | "onOrphansFound"
    | "onPluginCaptureConfirm"
    | "onPluginCaptureConflict"
  >;
} {
  const { out, policy } = ctx;
  const p = out.progress(`Fetching ${source}...`);
  const steps = createStepLogger(ctx.verbose);

  const { callbacks, logScanResults } = createInstallCallbacks({
    out,
    progress: p,
    onWarn: policy.onWarn,
    skipScan: policy.skipScan,
    agent: ctx.agent,
    yes: policy.yes,
    source,
    steps,
  });

  const forcedCaptureNames = new Set<string>();

  const captureCallbacks: Pick<
    InstallOptions,
    "onPluginCaptureConflict" | "onPluginCaptureConfirm"
  > = {
    ...(process.stdout.isTTY
      ? {
          async onPluginCaptureConflict(
            crossSource: CaptureBucket,
          ): Promise<"abort" | "force"> {
            p.pause();
            printCaptureConflict(crossSource, source);
            const { isCancel: isCancelPrompt } = await import("@clack/prompts");
            const { footerSelect: footerSel } = await import("../../ui/footer");
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
              p.resume();
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
            p.resume();
            return resolved;
          },
        }
      : {}),
    async onPluginCaptureConfirm(bucket: CaptureBucket): Promise<boolean> {
      if (policy.yes) return true;
      p.pause();
      printCaptureSummary(bucket, source, forcedCaptureNames);
      const { isCancel: isCancelPrompt } = await import("@clack/prompts");
      const { footerConfirm: footerConf } = await import("../../ui/footer");
      const proceed = await footerConf({
        message: `Capture these components into the plugin?`,
        initialValue: true,
      });
      if (isCancelPrompt(proceed) || proceed === false) {
        p.resume();
        return false;
      }
      p.resume();
      return true;
    },
  };

  const onOrphansFound = async (orphans: OrphanRecord[]): Promise<string[]> => {
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
  };

  return {
    progress: p,
    logScanResults,
    installOptions: {
      ...callbacks,
      ...captureCallbacks,
      onOrphansFound,
    },
  };
}

export { parseAlsoFlag };
