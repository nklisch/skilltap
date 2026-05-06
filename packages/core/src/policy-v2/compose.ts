import type { ConfigV2 } from "../schemas/config-v2";
import { err, ok, type Result, UserError } from "../types";
import { isTrusted } from "./trust-glob";
import type {
  CliFlagsV2,
  EffectivePolicyV2,
  EnvV2,
  SourceForPolicy,
} from "./types";

// Resolve `agent` from precedence: --no-agent > --agent > env > config.default.
// Returns the resolved boolean OR an error if config.agent.block forbids it.
function resolveAgent(
  config: ConfigV2,
  flags: CliFlagsV2,
  env?: EnvV2,
): Result<boolean, UserError> {
  let agent: boolean;
  if (flags.noAgent === true) agent = false;
  else if (flags.agent === true) agent = true;
  else if (env?.agent === true) agent = true;
  else agent = config.agent.default;

  if (agent && config.agent.block) {
    return err(
      new UserError(
        "--agent is blocked by config (agent.block = true). Set agent.block = false to enable.",
      ),
    );
  }
  return ok(agent);
}

function resolveYes(flags: CliFlagsV2, agent: boolean): boolean {
  if (flags.noYes === true) return false;
  if (flags.yes === true) return true;
  return agent;
}

function resolveScanMode(
  config: ConfigV2,
  flags: CliFlagsV2,
): EffectivePolicyV2["scanMode"] {
  if (flags.deep === true) return "semantic";
  return config.security.scan;
}

function resolveOnWarn(
  config: ConfigV2,
  flags: CliFlagsV2,
): EffectivePolicyV2["onWarn"] {
  if (flags.noStrict === true) return config.security.on_warn;
  if (flags.strict === true) return "fail";
  return config.security.on_warn;
}

function resolveScope(
  config: ConfigV2,
  flags: CliFlagsV2,
): EffectivePolicyV2["scope"] {
  if (flags.project === true) return "project";
  if (flags.global === true) return "global";
  return config.defaults.scope;
}

// Compose a base v2 policy from config + flags + env. Per-source trust-list
// resolution happens in composeV2ForSource.
export function composeV2(
  config: ConfigV2,
  flags: CliFlagsV2,
  env?: EnvV2,
): Result<EffectivePolicyV2, UserError> {
  const agentResult = resolveAgent(config, flags, env);
  if (!agentResult.ok) return agentResult;
  const agent = agentResult.value;

  return ok({
    yes: resolveYes(flags, agent),
    agent,
    scope: resolveScope(config, flags),
    also: config.defaults.also,
    scanMode: resolveScanMode(config, flags),
    onWarn: resolveOnWarn(config, flags),
    skipScan: flags.skipScan === true,
    trusted: false,
  });
}

// Per-source variant. After computing the base policy, applies the trust list:
// any matching pattern → trusted=true and scanMode forced to "none".
export function composeV2ForSource(
  config: ConfigV2,
  flags: CliFlagsV2,
  source: SourceForPolicy,
  env?: EnvV2,
): Result<EffectivePolicyV2, UserError> {
  const base = composeV2(config, flags, env);
  if (!base.ok) return base;

  if (isTrusted(config.security.trust, source)) {
    return ok({
      ...base.value,
      trusted: true,
      scanMode: "none",
    });
  }
  return base;
}
