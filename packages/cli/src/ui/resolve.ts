import { isCancel, log } from "@clack/prompts";
import {
  type AgentAdapter,
  type Config,
  type InstalledSkill,
  findProjectRoot,
  loadInstalled,
  resolveAgent,
  saveConfig,
  VALID_AGENT_IDS,
} from "@skilltap/core";
import { agentError } from "./agent-out";
import { errorLine } from "./format";
import { selectAgent, selectScope } from "./prompts";

export type ScopeArgs = {
  project?: boolean;
  global?: boolean;
};

/** Resolve scope from CLI flags, config default, or interactive prompt. */
export async function resolveScope(
  args: ScopeArgs,
  config?: Config,
): Promise<{ scope: "global" | "project"; projectRoot?: string }> {
  let scope: "global" | "project";
  let projectRoot: string | undefined;

  if (args.project) {
    scope = "project";
    projectRoot = await findProjectRoot();
  } else if (args.global) {
    scope = "global";
  } else if (config?.defaults.scope) {
    scope = config.defaults.scope as "global" | "project";
    if (scope === "project") projectRoot = await findProjectRoot();
  } else {
    const chosen = await selectScope();
    if (isCancel(chosen)) process.exit(2);
    scope = chosen as "global" | "project";
    if (scope === "project") projectRoot = await findProjectRoot();
  }

  return { scope, projectRoot };
}

/** Parse --also flag with agent validation, falling back to config defaults. */
export function parseAlsoFlag(
  alsoArg: string | undefined,
  config?: Config,
): string[] {
  if (!alsoArg) return config?.defaults.also ?? [];

  const also: string[] = [];
  const agents = alsoArg
    .split(",")
    .map((a) => a.trim())
    .filter(Boolean);
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

/** Resolve agent for interactive mode: prompt user if needed, save choice to config. */
export async function resolveAgentInteractive(
  config: Config,
): Promise<AgentAdapter | undefined> {
  const agentResult = await resolveAgent(config, async (detected) => {
    const chosen = await selectAgent(detected);
    if (isCancel(chosen)) return null;
    config.security.agent = (chosen as AgentAdapter).cliName;
    await saveConfig(config);
    return chosen as AgentAdapter;
  });

  if (agentResult.ok) {
    return agentResult.value ?? undefined;
  }
  log.warn(agentResult.error.message);
  return undefined;
}

/** Resolve agent for agent mode: exit if semantic scan requires agent but none configured. */
export async function resolveAgentForAgentMode(
  config: Config,
): Promise<AgentAdapter> {
  const agentResult = await resolveAgent(config);
  if (!agentResult.ok || !agentResult.value) {
    agentError(
      "Agent mode requires security.agent to be set for semantic scanning. Run 'skilltap config' to configure.",
    );
    process.exit(1);
  }
  return agentResult.value;
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
  const globalResult = await loadInstalled();
  if (!globalResult.ok) {
    errorLine(globalResult.error.message);
    process.exit(1);
  }

  const projectRoot = await findProjectRoot().catch(() => undefined);
  const projectResult = projectRoot ? await loadInstalled(projectRoot) : null;

  const allSkills = [
    ...globalResult.value.skills,
    ...(projectResult?.ok ? projectResult.value.skills : []),
  ];

  const predicate = opts?.filter
    ? (s: InstalledSkill) => s.name === name && opts.filter!(s)
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
