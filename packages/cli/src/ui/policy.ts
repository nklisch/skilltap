import {
  type CliFlags,
  type Config,
  composePolicy,
  type EffectivePolicy,
  loadConfig,
} from "@skilltap/core";
import { errorLine } from "./format";

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
    errorLine(policyResult.error.message);
    process.exit(1);
  }

  return { config, policy: policyResult.value };
}
