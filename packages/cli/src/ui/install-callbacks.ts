import { isCancel, spinner } from "@clack/prompts";
import { footerConfirm as confirm } from "./footer";
import type {
  AgentAdapter,
  InstallOptions,
  ScannedSkill,
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
  onWarn: "fail" | "prompt" | "ask" | "skip";
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
          const proceed = await confirmInstall(skillName);
          if (isCancel(proceed) || proceed === false) process.exit(2);
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
          const proceed = await confirmInstall(skillName);
          if (isCancel(proceed) || proceed === false) process.exit(2);
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
        if (isCancel(selected)) process.exit(2);
        return selected as string[];
      });
    },

    onSelectTap: async (matches: TapEntry[]): Promise<TapEntry | null> =>
      withSpinnerPaused(s, async () => {
        const chosen = await selectTap(matches);
        if (isCancel(chosen)) process.exit(2);
        return chosen as TapEntry;
      }),

    onAlreadyInstalled: async (name: string): Promise<"update" | "abort"> => {
      if (yes) return "update";
      return withSpinnerPaused(s, async () => {
        const proceed = await confirm({
          message: `${name} is already installed. Update it instead?`,
          initialValue: true,
        });
        if (isCancel(proceed) || proceed === false) return "abort";
        return "update";
      });
    },

    onOfferSemantic: agent
      ? async (): Promise<boolean> => {
          return withSpinnerPaused(
            s,
            async () => {
              const answer = await offerSemanticScan();
              if (isCancel(answer)) return false;
              return answer as boolean;
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
            if (isCancel(proceed) || proceed === false) process.exit(2);
            return true;
          }),

    onDeepScan: async (count: number): Promise<boolean> =>
      withSpinnerPaused(s, async () => {
        const proceed = await confirm({
          message: `Found ${count} SKILL.md at non-standard path(s). Continue?`,
          initialValue: true,
        });
        if (isCancel(proceed) || proceed === false) process.exit(2);
        return true;
      }),
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
