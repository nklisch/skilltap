import { confirm, isCancel } from "@clack/prompts";
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

type Spinner = {
  start: (msg?: string) => void;
  stop: (msg?: string, code?: number) => void;
  message: (msg: string) => void;
};

export type CallbackContext = {
  spinner: Spinner;
  onWarn: "fail" | "ask" | "skip";
  skipScan: boolean;
  agent: AgentAdapter | undefined;
  yes: boolean;
};

async function withSpinnerPaused<T>(
  s: Spinner,
  fn: () => Promise<T>,
  resumeMsg = "Installing...",
): Promise<T> {
  s.stop();
  try {
    return await fn();
  } finally {
    s.start(resumeMsg);
  }
}

function makeWarnCallback<W>(
  s: Spinner,
  onWarn: string,
  printFn: (warnings: W[], skillName: string) => void,
  failMsg: (skillName: string) => string,
): (warnings: W[], skillName: string) => Promise<boolean> {
  return async (warnings, skillName) =>
    withSpinnerPaused(s, async () => {
      printFn(warnings, skillName);
      if (onWarn === "fail") {
        errorLine(failMsg(skillName));
        process.exit(1);
      }
      const proceed = await confirmInstall(skillName);
      if (isCancel(proceed) || proceed === false) process.exit(2);
      return true;
    });
}

export function createInstallCallbacks(
  ctx: CallbackContext,
): Pick<
  InstallOptions,
  | "onWarnings"
  | "onSelectSkills"
  | "onSelectTap"
  | "onAlreadyInstalled"
  | "onSemanticWarnings"
  | "onOfferSemantic"
  | "onSemanticProgress"
  | "onConfirmInstall"
  | "onDeepScan"
> {
  const { spinner: s, onWarn, skipScan, agent, yes } = ctx;

  const warningsCallback = makeWarnCallback(
    s,
    onWarn,
    printWarnings,
    (name) => `Security warnings found in ${name} — aborting (--strict / on_warn=fail)`,
  );

  const semanticWarningsCallback = makeWarnCallback(
    s,
    onWarn,
    printSemanticWarnings,
    (name) => `Semantic warnings found in ${name} — aborting (--strict / on_warn=fail)`,
  );

  return {
    onWarnings: skipScan ? undefined : warningsCallback,

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

    onSemanticWarnings: agent ? semanticWarningsCallback : undefined,

    onOfferSemantic: agent
      ? async (): Promise<boolean> => {
          if (!agent) return false;
          return withSpinnerPaused(s, async () => {
            const answer = await offerSemanticScan();
            if (isCancel(answer)) return false;
            return answer as boolean;
          }, "Running semantic scan...");
        }
      : undefined,

    onSemanticProgress: agent
      ? (completed: number, total: number): void => {
          s.message(`Scanning chunk ${completed}/${total}...`);
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
}
