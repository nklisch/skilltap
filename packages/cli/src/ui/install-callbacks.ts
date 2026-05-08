import { spinner } from "@clack/prompts";
import type {
  AgentAdapter,
  CaptureBucket,
  InstallOptions,
  Output,
  PluginManifest,
  Progress,
  ScannedSkill,
  StaticWarning,
  TapEntry,
} from "@skilltap/core";
import { footerConfirm as confirm, footerSelect as select } from "./footer";
import type { StepLogger } from "./install-steps";
import { pluginComponentSummary } from "./plugin-format";
import {
  confirmInstall,
  confirmReadyInstall,
  offerSemanticScan,
  selectSkills,
  selectTap,
} from "./prompts";
import { printSemanticWarnings, printWarnings } from "./scan";

function truncate(str: string, max: number): string {
  return str.length <= max ? str : `${str.slice(0, max - 1)}…`;
}

/**
 * Render the cross-source conflict block — called before the select prompt
 * that asks the user whether to abort or force.
 */
export function printCaptureConflict(
  matches: CaptureBucket,
  pluginRepo: string | null,
): void {
  const pluginLabel = pluginRepo ?? "(no recorded source)";

  log.warn(
    `Plugin wants to replace standalone components from a DIFFERENT source.`,
  );

  if (matches.skills.length > 0) {
    log.info(`  Skills (${matches.skills.length}):`);
    for (const c of matches.skills) {
      const standaloneLabel = c.standalone.repo
        ? c.standalone.repo
        : c.standalone.scope === "linked"
          ? `linked at ${c.standalone.path ?? "(unknown path)"}`
          : "(no recorded source)";
      log.info(`    • ${c.standalone.name}`);
      log.info(`        standalone: ${standaloneLabel}`);
      log.info(`        plugin:     ${pluginLabel}`);
      log.warn(`        Different sources. The plugin's version would replace the standalone's content.`);
    }
  }

  if (matches.mcpServers.length > 0) {
    log.info(`  MCP servers (${matches.mcpServers.length}):`);
    for (const c of matches.mcpServers) {
      const standaloneLabel = c.standalone.source ?? "(no recorded source)";
      const slug = c.standalone.name.split(":")[1] ?? "";
      const keyDisplay = slug
        ? ` (slug=${slug} → ${c.standalone.name})`
        : "";
      log.info(`    • ${c.serverName}`);
      log.info(`        standalone: ${standaloneLabel}${keyDisplay}`);
      log.info(`        plugin:     ${pluginLabel}`);
    }
  }

  log.warn(`This is silent substitution. Choose carefully.`);
}

/**
 * Render the capture summary — called before the confirm prompt that asks
 * whether to proceed with the transfer of ownership.
 *
 * `forced` is the set of names whose ownership would transfer via a cross-source
 * override (i.e., the user said "force" in the conflict prompt). These rows
 * are rendered with a [FORCED] prefix so the user gets one last look.
 */
export function printCaptureSummary(
  matches: CaptureBucket,
  pluginName: string,
  forced: Set<string>,
): void {
  log.info(`Plugin "${pluginName}" wants to take ownership of:`);

  if (matches.skills.length > 0) {
    log.info(`  Skills (${matches.skills.length}):`);
    for (const c of matches.skills) {
      const skill = c.standalone;
      const isForced = forced.has(skill.name);
      const sourceLabel =
        skill.scope === "linked"
          ? `linked at ${skill.path ?? "(unknown path)"}`
          : skill.repo
            ? `${skill.repo}, ${skill.scope}`
            : skill.scope;
      const forcedTag = isForced ? ` [FORCED]` : "";
      log.info(`    • ${skill.name}${forcedTag}  (was: ${sourceLabel})`);
    }
  }

  if (matches.mcpServers.length > 0) {
    log.info(`  MCP servers (${matches.mcpServers.length}):`);
    for (const c of matches.mcpServers) {
      const isForced = forced.has(c.serverName);
      const targets = c.standalone.targets.join(", ");
      const targetsLabel = targets ? ` → ${targets}` : "";
      const forcedTag = isForced ? ` [FORCED]` : "";
      log.info(
        `    • ${c.serverName}${forcedTag}  (was: ${c.standalone.name}${targetsLabel})`,
      );
    }
  }

  log.info(
    `These standalones will be removed from skilltap.toml and skilltap.lock.`,
  );
  log.info(`The plugin's bundled versions will replace them on disk.`);
}

export type CallbackContext = {
  out: Output;
  progress: Progress;
  onWarn: "fail" | "prompt" | "allow" | "ask" | "skip";
  skipScan: boolean;
  agent: AgentAdapter | undefined;
  yes: boolean;
  source: string;
  steps: StepLogger;
};

async function withProgressPaused<T>(
  p: Progress,
  fn: () => Promise<T>,
  resumeMsg?: string,
): Promise<T> {
  p.pause();
  try {
    return await fn();
  } finally {
    if (resumeMsg) p.resume();
  }
}

export function createInstallCallbacks(ctx: CallbackContext): {
  callbacks: Pick<
    InstallOptions,
    | "onWarnings"
    | "onSelectSkills"
    | "onSelectTap"
    | "onAlreadyInstalled"
    | "onSemanticWarnings"
    | "onOfferSemantic"
    | "onSemanticProgress"
    | "onStaticScanStart"
    | "onSemanticScanStart"
    | "onConfirmInstall"
    | "onDeepScan"
    | "onPluginDetected"
    | "onPluginWarnings"
    | "onPluginConfirm"
  >;
  logScanResults(): void;
} {
  const { out, progress: p, onWarn, skipScan, agent, yes, source, steps } = ctx;

  let staticStarted = false;
  let hadStaticWarnings = false;
  let semanticStarted = false;
  let hadSemanticWarnings = false;
  let semSpinner: ReturnType<typeof spinner> | null = null;

  const callbacks: ReturnType<typeof createInstallCallbacks>["callbacks"] = {
    onStaticScanStart: skipScan
      ? undefined
      : (_skillName: string): void => {
          // Fetch phase complete — switch from fetch spinner to scan step
          p.pause();
          steps.fetched(source);
          staticStarted = true;
        },

    onWarnings: skipScan
      ? undefined
      : async (warnings, skillName): Promise<boolean> => {
          hadStaticWarnings = true;
          printWarnings(warnings, skillName, out);
          if (onWarn === "fail") {
            out.error(
              `Security warnings found in ${skillName} — aborting (--strict / on_warn=fail)`,
            );
            process.exit(1);
          }
          if (onWarn === "allow") {
            // Warnings logged for visibility; auto-continue
            return true;
          }
          const proceed = await confirmInstall(skillName);
          if (proceed === false) process.exit(2);
          return true;
        },

    onSemanticScanStart: agent
      ? (skillName: string): void => {
          // Static scan phase complete — log clean result then start semantic spinner
          if (!hadStaticWarnings) steps.staticScanClean();
          semSpinner = spinner();
          semSpinner.start(
            `Semantic scan of ${skillName} via ${agent.name}...`,
          );
          semanticStarted = true;
        }
      : undefined,

    onSemanticProgress: agent
      ? (
          completed: number,
          total: number,
          score: number,
          reason: string,
        ): void => {
          const threshold = 5;
          const flag = score >= threshold ? ` — ⚠ ${truncate(reason, 60)}` : "";
          semSpinner?.message(
            `Semantic scan: chunk ${completed}/${total}${flag}`,
          );
        }
      : undefined,

    onSemanticWarnings: agent
      ? async (warnings, skillName): Promise<boolean> => {
          hadSemanticWarnings = true;
          if (semSpinner) {
            semSpinner.stop();
            semSpinner = null;
          }
          printSemanticWarnings(warnings, skillName, out);
          if (onWarn === "fail") {
            out.error(
              `Semantic warnings found in ${skillName} — aborting (--strict / on_warn=fail)`,
            );
            process.exit(1);
          }
          if (onWarn === "allow") {
            // Warnings logged for visibility; auto-continue
            return true;
          }
          const proceed = await confirmInstall(skillName);
          if (proceed === false) process.exit(2);
          return true;
        }
      : undefined,

    onSelectSkills: async (skills: ScannedSkill[]): Promise<string[]> => {
      if (yes || skills.length === 1) {
        if (yes && skills.length > 1) {
          p.update(`Auto-selecting all ${skills.length} skills (--yes)`);
        }
        return skills.map((sk) => sk.name);
      }
      return withProgressPaused(p, async () => {
        const selected = await selectSkills(skills);
        return selected;
      });
    },

    onSelectTap: async (matches: TapEntry[]): Promise<TapEntry | null> =>
      withProgressPaused(p, async () => {
        const chosen = await selectTap(matches);
        return chosen;
      }),

    onAlreadyInstalled: async (name: string): Promise<"update" | "abort"> => {
      if (yes) return "update";
      return withProgressPaused(p, async () => {
        const { isCancel } = await import("@clack/prompts");
        const proceed = await confirm({
          message: `${name} is already installed. Update it instead?`,
          initialValue: true,
        });
        if (isCancel(proceed)) process.exit(130);
        if (proceed === false) return "abort";
        return "update";
      });
    },

    onOfferSemantic: agent
      ? async (): Promise<boolean> => {
          return withProgressPaused(
            p,
            async () => {
              const answer = await offerSemanticScan();
              return answer;
            },
            "Starting semantic scan...",
          );
        }
      : undefined,

    onConfirmInstall: yes
      ? undefined
      : async (skillNames: string[]): Promise<boolean> =>
          withProgressPaused(p, async () => {
            const proceed = await confirmReadyInstall(skillNames);
            if (proceed === false) process.exit(2);
            return true;
          }),

    onDeepScan: async (count: number): Promise<boolean> =>
      withProgressPaused(p, async () => {
        const { isCancel } = await import("@clack/prompts");
        const proceed = await confirm({
          message: `Found ${count} SKILL.md at non-standard path(s). Continue?`,
          initialValue: true,
        });
        if (isCancel(proceed)) process.exit(130);
        if (proceed === false) process.exit(2);
        return true;
      }),

    onPluginDetected: async (
      manifest: PluginManifest,
    ): Promise<"plugin" | "skills-only" | "cancel"> => {
      if (yes) return "plugin";
      return withProgressPaused(p, async () => {
        const { isCancel: isCancelPrompt } = await import("@clack/prompts");
        const summary = pluginComponentSummary(manifest);

        const decision = await select({
          message: `Plugin detected: ${manifest.name} (${manifest.format}) — ${summary}`,
          options: [
            {
              value: "plugin" as const,
              label: "Install as plugin",
              hint: "skills + MCP servers + agents",
            },
            {
              value: "skills-only" as const,
              label: "Install skills only",
              hint: "ignore MCP servers and agents",
            },
            { value: "cancel" as const, label: "Cancel" },
          ],
        });
        if (isCancelPrompt(decision)) process.exit(130);
        return decision as "plugin" | "skills-only" | "cancel";
      });
    },

    onPluginWarnings: skipScan
      ? undefined
      : async (
          warnings: StaticWarning[],
          pluginName: string,
        ): Promise<boolean> => {
          return withProgressPaused(p, async () => {
            printWarnings(warnings, pluginName, out);
            if (onWarn === "fail") {
              out.error(
                `Security warnings found in plugin ${pluginName} — aborting (--strict / on_warn=fail)`,
              );
              process.exit(1);
            }
            if (onWarn === "allow") return true;
            const proceed = await confirmInstall(pluginName);
            if (proceed === false) process.exit(2);
            return true;
          });
        },

    onPluginConfirm: yes
      ? undefined
      : async (manifest: PluginManifest): Promise<boolean> => {
          return withProgressPaused(p, async () => {
            const proceed = await confirmReadyInstall([manifest.name]);
            if (proceed === false) process.exit(2);
            return true;
          });
        },
  };

  function logScanResults(): void {
    if (semSpinner) {
      semSpinner.stop();
      semSpinner = null;
      // biome-ignore lint/style/noNonNullAssertion: semSpinner exists ⇒ semantic scan started ⇒ agent set
      if (!hadSemanticWarnings) steps.semanticScanClean(agent!.name);
    } else if (staticStarted && !hadStaticWarnings && !semanticStarted) {
      steps.staticScanClean();
    }
  }

  return { callbacks, logScanResults };
}
