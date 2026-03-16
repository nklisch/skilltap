import {
  type CliFlags,
  type Config,
  type EffectivePolicy,
  composePolicy,
  loadConfig,
} from "@skilltap/core";
import { agentError } from "./agent-out";
import { errorLine } from "./format";

export async function isAgentMode(): Promise<boolean> {
  const configResult = await loadConfig();
  return configResult.ok && configResult.value["agent-mode"].enabled;
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
