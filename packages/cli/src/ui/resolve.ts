import { log } from "@clack/prompts";
import {
  type AgentAdapter,
  type Config,
  type EffectivePolicy,
  findProjectRoot,
  type InstalledSkill,
  isInGitRepo,
  loadSkillState,
  type Output,
  resolveAgent,
  saveConfig,
  VALID_AGENT_IDS,
} from "@skilltap/core";

export async function tryFindProjectRoot(): Promise<string | undefined> {
  return findProjectRoot().catch(() => undefined);
}

import { errorLine } from "../output/write-helpers";
import { selectAgent } from "./prompts";

export type ScopeArgs = {
  scope?: "project" | "global";
};

/**
 * Resolve scope from CLI flag, config default, or smart inference.
 *
 * Smart default: when no flag and no config default is set, infer the scope
 * from the cwd's git context. Inside a git repo → project; outside → global.
 * No interactive prompt — the inferred scope is reported via the returned
 * object (caller decides whether to surface it). Pass --scope project|global
 * to override the inference.
 */
export async function resolveScope(
  args: ScopeArgs,
  config?: Config,
): Promise<{
  scope: "global" | "project";
  projectRoot?: string;
  inferred?: boolean;
}> {
  let scope: "global" | "project";
  let projectRoot: string | undefined;
  let inferred = false;

  if (args.scope === "project") {
    scope = "project";
    projectRoot = await findProjectRoot();
  } else if (args.scope === "global") {
    scope = "global";
  } else if (config?.defaults.scope) {
    scope = config.defaults.scope as "global" | "project";
    if (scope === "project") projectRoot = await findProjectRoot();
  } else {
    const gitRoot = await isInGitRepo();
    if (gitRoot) {
      scope = "project";
      projectRoot = gitRoot;
    } else {
      scope = "global";
    }
    inferred = true;
  }

  return { scope, projectRoot, inferred };
}

/**
 * Collect all values of a repeatable string flag (e.g. `--also a --also b`)
 * from a raw argv array. citty's mri parser only keeps the LAST occurrence of
 * a string flag in `args`, so for repeatable string flags we walk rawArgs
 * directly. Supports both `--flag value` and `--flag=value`.
 */
export function collectRepeatedFlag(
  rawArgs: readonly string[],
  flag: string,
): string[] | undefined {
  const long = `--${flag}`;
  const eqPrefix = `${long}=`;
  const values: string[] = [];
  let seen = false;
  for (let i = 0; i < rawArgs.length; i++) {
    const arg = rawArgs[i];
    if (arg === long) {
      seen = true;
      const next = rawArgs[i + 1];
      if (next !== undefined && !next.startsWith("-")) {
        values.push(next);
        i++;
      }
    } else if (arg?.startsWith(eqPrefix)) {
      seen = true;
      values.push(arg.slice(eqPrefix.length));
    }
  }
  return seen ? values : undefined;
}

/**
 * Returns true if `rawArgs` contains the literal flag `--<flag>`. Citty/mri
 * intercepts `--no-*` patterns as negations of the base flag (so `--no-capture`
 * sets `args.capture = false` rather than `args["no-capture"] = true`). When
 * the flag NAME contains a hyphenated `no-` segment that is meant to be read
 * literally — e.g. the design-mandated `--no-capture` boolean — call this
 * helper on `rawArgs` instead of trusting the parsed `args` object.
 */
export function hasRawFlag(rawArgs: readonly string[], flag: string): boolean {
  const long = `--${flag}`;
  for (const arg of rawArgs) {
    if (arg === long) return true;
  }
  return false;
}

/**
 * Parse repeatable --also flag with agent validation. Falls back to config
 * defaults when the flag is absent.
 */
export function parseAlsoFlag(
  alsoArg: string | string[] | undefined,
  configDefaultAlso: readonly string[],
): string[] {
  if (alsoArg === undefined) return [...configDefaultAlso];

  const agents = Array.isArray(alsoArg) ? alsoArg : [alsoArg];
  const also: string[] = [];
  for (const agent of agents) {
    if (!VALID_AGENT_IDS.includes(agent)) {
      errorLine(
        `Unknown agent: "${agent}"`,
        `Valid agents: ${VALID_AGENT_IDS.join(", ")}`,
      );
      process.exit(1);
    }
    also.push(agent);
  }
  return also;
}

/** Resolve whether to run semantic scan and which agent to use, for interactive mode. */
export async function resolveSemanticInteractive(
  policy: EffectivePolicy,
  args: { semantic: boolean },
  config: Config,
): Promise<{ runSemantic: boolean; agent: AgentAdapter | undefined }> {
  const runSemantic = policy.scanMode === "semantic" || args.semantic;
  let agent: AgentAdapter | undefined;
  if (runSemantic) {
    agent = await resolveAgentInteractive(config);
    if (!agent) {
      log.warn("No agent CLI found on PATH. Skipping semantic scan.");
    }
  }
  return { runSemantic, agent };
}

/** Resolve agent for interactive mode: prompt user if needed, save choice to config. */
export async function resolveAgentInteractive(
  config: Config,
): Promise<AgentAdapter | undefined> {
  const agentResult = await resolveAgent(config, async (detected) => {
    const chosen = await selectAgent(detected);
    config.scanner.agent_cli = chosen.cliName;
    await saveConfig(config);
    return chosen;
  });

  if (agentResult.ok) {
    return agentResult.value ?? undefined;
  }
  log.warn(agentResult.error.message);
  return undefined;
}

export function validateScopeArg(
  scopeArg: string | undefined,
  out: Output,
  options: { required?: boolean } = {},
): "project" | "global" | undefined {
  if (scopeArg === undefined) {
    if (options.required) {
      out.error("Specify target scope: --scope project|global");
      process.exit(1);
    }
    return undefined;
  }
  if (scopeArg !== "project" && scopeArg !== "global") {
    out.error(
      `Invalid --scope value '${scopeArg}'. Use 'project' or 'global'.`,
    );
    process.exit(1);
  }
  return scopeArg;
}

/** Load installed skills and find by name, or exit with a contextual error. */
export async function getInstalledSkillOrExit(
  name: string,
  opts?: {
    filter?: (skill: InstalledSkill) => boolean;
    notFoundMessage?: string;
    notFoundHint?: string;
  },
): Promise<InstalledSkill> {
  const globalResult = await loadSkillState();
  if (!globalResult.ok) {
    errorLine(globalResult.error.message);
    process.exit(1);
  }

  const projectRoot = await tryFindProjectRoot();
  const projectResult = projectRoot ? await loadSkillState(projectRoot) : null;

  const allSkills = [
    ...globalResult.value,
    ...(projectResult?.ok ? projectResult.value : []),
  ];

  const predicate = opts?.filter
    ? // biome-ignore lint/style/noNonNullAssertion: opts.filter checked truthy in ternary above
      (s: InstalledSkill) => s.name === name && opts.filter!(s)
    : (s: InstalledSkill) => s.name === name;

  const skill = allSkills.find(predicate);
  if (!skill) {
    errorLine(
      opts?.notFoundMessage ?? `Skill '${name}' is not installed`,
      opts?.notFoundHint,
    );
    process.exit(1);
  }

  return skill;
}
