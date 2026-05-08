import type { Config, EffectivePolicy, Output } from "@skilltap/core";
import { loadPolicyOrExit } from "../../ui/policy";
import { resolveScope, tryFindProjectRoot } from "../../ui/resolve";
import { setupOutput } from "../../ui/setup";

export interface RemoveContext {
  out: Output;
  config: Config;
  policy: EffectivePolicy;
  projectRoot: string | undefined;
  scope: "global" | "project";
}

export async function setupRemoveContext(args: {
  json?: boolean;
  project?: boolean;
  global?: boolean;
  yes?: boolean;
}): Promise<RemoveContext> {
  const out = setupOutput(args);
  const { config, policy } = await loadPolicyOrExit({
    yes: args.yes,
    project: args.project,
    global: args.global,
  });
  const projectRoot = await tryFindProjectRoot();
  const { scope } = await resolveScope(args, config);
  return { out, config, policy, projectRoot, scope };
}
