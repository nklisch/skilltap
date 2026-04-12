import { spinner } from "@clack/prompts";
import { footerConfirm as confirm, footerSelect as select } from "./footer";
import type {
  AgentAdapter,
  InstallOptions,
  PluginManifest,
  ScannedSkill,
  StaticWarning,
  TapEntry,
} from "@skilltap/core";
import { errorLine } from "./format";
import {
  confirmInstall,
  confirmReadyInstall,
  offerSemanticScan,
  selectSkills,
  selectTap,
} from "./prompts";
import { printSemanticWarnings, printWarnings } from "./scan";
import type { StepLogger } from "./install-steps";
import { pluginComponentSummary } from "./plugin-format";

function truncate(str: string, max: number): string {
  return str.length <= max ? str : `${str.slice(0, max - 1)}…`;
}

type Spinner = {
  start: (msg?: string) => void;
  stop: (msg?: string, code?: number) => void;
  message: (msg: string) => void;
};

export type CallbackContext = {
  spinner: Spinner;
  onWarn: "fail" | "prompt" | "allow" | "ask" | "skip";
  skipScan: boolean;
  agent: AgentAdapter | undefined;
  yes: boolean;
  source: string;
  steps: StepLogger;
};

async function withSpinnerPaused<T>(
  s: Spinner,
  fn: () => Promise<T>,
  resumeMsg?: string,
): Promise<T> {
  s.stop();
  try {
    return await fn();
  } finally {
    if (resumeMsg) s.start(resumeMsg);
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
  const { spinner: s, onWarn, skipScan, agent, yes, source, steps } = ctx;

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
          s.stop();
          steps.fetched(source);
          staticStarted = true;
        },

    onWarnings: skipScan
      ? undefined
      : async (warnings, skillName): Promise<boolean> => {
          hadStaticWarnings = true;
          printWarnings(warnings, skillName);
          if (onWarn === "fail") {
            errorLine(
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
      ? (completed: number, total: number, score: number, reason: string): void => {
          const threshold = 5;
          const flag =
            score >= threshold ? ` — ⚠ ${truncate(reason, 60)}` : "";
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
          printSemanticWarnings(warnings, skillName);
          if (onWarn === "fail") {
            errorLine(
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
          s.message(`Auto-selecting all ${skills.length} skills (--yes)`);
        }
        return skills.map((sk) => sk.name);
      }
      return withSpinnerPaused(s, async () => {
        const selected = await selectSkills(skills);
        return selected;
      });
    },

    onSelectTap: async (matches: TapEntry[]): Promise<TapEntry | null> =>
      withSpinnerPaused(s, async () => {
        const chosen = await selectTap(matches);
        return chosen;
      }),

    onAlreadyInstalled: async (name: string): Promise<"update" | "abort"> => {
      if (yes) return "update";
      return withSpinnerPaused(s, async () => {
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
          return withSpinnerPaused(
            s,
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
          withSpinnerPaused(s, async () => {
            const proceed = await confirmReadyInstall(skillNames);
            if (proceed === false) process.exit(2);
            return true;
          }),

    onDeepScan: async (count: number): Promise<boolean> =>
      withSpinnerPaused(s, async () => {
        const { isCancel } = await import("@clack/prompts");
        const proceed = await confirm({
          message: `Found ${count} SKILL.md at non-standard path(s). Continue?`,
          initialValue: true,
        });
        if (isCancel(proceed)) process.exit(130);
        if (proceed === false) process.exit(2);
        return true;
      }),

    onPluginDetected: async (manifest: PluginManifest): Promise<"plugin" | "skills-only" | "cancel"> => {
      if (yes) return "plugin";
      return withSpinnerPaused(s, async () => {
        const { isCancel: isCancelPrompt } = await import("@clack/prompts");
        const summary = pluginComponentSummary(manifest);

        const decision = await select({
          message: `Plugin detected: ${manifest.name} (${manifest.format}) — ${summary}`,
          options: [
            { value: "plugin" as const, label: "Install as plugin", hint: "skills + MCP servers + agents" },
            { value: "skills-only" as const, label: "Install skills only", hint: "ignore MCP servers and agents" },
            { value: "cancel" as const, label: "Cancel" },
          ],
        });
        if (isCancelPrompt(decision)) process.exit(130);
        return decision as "plugin" | "skills-only" | "cancel";
      });
    },

    onPluginWarnings: skipScan
      ? undefined
      : async (warnings: StaticWarning[], pluginName: string): Promise<boolean> => {
          return withSpinnerPaused(s, async () => {
            printWarnings(warnings, pluginName);
            if (onWarn === "fail") {
              errorLine(`Security warnings found in plugin ${pluginName} — aborting (--strict / on_warn=fail)`);
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
          return withSpinnerPaused(s, async () => {
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
      if (!hadSemanticWarnings) steps.semanticScanClean(agent!.name);
    } else if (staticStarted && !hadStaticWarnings && !semanticStarted) {
      steps.staticScanClean();
    }
  }

  return { callbacks, logScanResults };
}
