import {
  type CliFlags,
  type Config,
  composePolicy,
  type EffectivePolicy,
  isAgentEnv,
  loadConfig,
} from "@skilltap/core";
import { agentError } from "./agent-out";
import { errorLine } from "./format";

export async function isAgentMode(): Promise<boolean> {
  // Precedence matches composePolicy: --agent flag > env var > legacy config.
  // Commands that don't go through composePolicy (disable, enable, toggle,
  // plugin/skills/tap info-only paths) call this helper, so the flag must
  // be detected directly from argv to honor the documented precedence.
  if (hasAgentFlag()) return true;
  if (isAgentEnv()) return true;
  const configResult = await loadConfig();
  return configResult.ok && configResult.value["agent-mode"].enabled;
}

function hasAgentFlag(): boolean {
  for (const arg of process.argv) {
    if (arg === "--agent" || arg === "--agent=true" || arg === "--agent=1") {
      return true;
    }
  }
  return false;
}

export async function loadPolicyOrExit(
  flags: CliFlags,
): Promise<{ config: Config; policy: EffectivePolicy }> {
  const configResult = await loadConfig();
  if (!configResult.ok) {
    errorLine(configResult.error.message, configResult.error.hint);
    process.exit(1);
  }
  const config = configResult.value;

  const policyResult = composePolicy(config, flags);
  if (!policyResult.ok) {
    if (config["agent-mode"].enabled) {
      agentError(policyResult.error.message);
    } else {
      errorLine(policyResult.error.message);
    }
    process.exit(1);
  }

  return { config, policy: policyResult.value };
}
