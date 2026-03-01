import { isCancel } from "@clack/prompts";
import { type Config, findProjectRoot, VALID_AGENT_IDS } from "@skilltap/core";
import { errorLine } from "./format";
import { selectScope } from "./prompts";

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
