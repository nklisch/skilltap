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
  /** True iff --scope was explicitly provided (vs. inferred from cwd). */
  scopeProvided: boolean;
}

export async function setupRemoveContext(args: {
  json?: boolean;
  scope?: string;
  yes?: boolean;
}): Promise<RemoveContext> {
  const out = setupOutput(args);

  const scopeArg = args.scope;
  if (
    scopeArg !== undefined &&
    scopeArg !== "project" &&
    scopeArg !== "global"
  ) {
    out.error(
      `Invalid --scope value '${scopeArg}'. Use 'project' or 'global'.`,
    );
    process.exit(1);
  }
  const scopeFlag = scopeArg as "project" | "global" | undefined;

  const { config, policy } = await loadPolicyOrExit({
    yes: args.yes,
    scope: scopeFlag,
  });
  const projectRoot = await tryFindProjectRoot();
  const { scope, inferred } = await resolveScope({ scope: scopeFlag }, config);
  const scopeProvided = scopeFlag !== undefined || !inferred;
  return {
    out,
    config,
    policy,
    projectRoot,
    scope,
    scopeProvided,
  };
}
