import type { Config } from "../schemas/config";
import { ok, type Result, UserError } from "../types";
import { isTrusted } from "./trust-glob";
import type { CliFlags, EffectivePolicy, SourceForPolicy } from "./types";

function resolveYes(flags: CliFlags): boolean {
  if (flags.noYes === true) return false;
  return flags.yes === true;
}

function resolveScanMode(
  config: Config,
  flags: CliFlags,
): EffectivePolicy["scanMode"] {
  if (flags.deep === true) return "semantic";
  return config.security.scan;
}

function resolveOnWarn(
  config: Config,
  flags: CliFlags,
): EffectivePolicy["onWarn"] {
  if (flags.strict === true) return "fail";
  return config.security.on_warn;
}

function resolveScope(
  config: Config,
  flags: CliFlags,
): EffectivePolicy["scope"] {
  if (flags.scope === "project") return "project";
  if (flags.scope === "global") return "global";
  return config.defaults.scope;
}

export function composePolicy(
  config: Config,
  flags: CliFlags,
): Result<EffectivePolicy, UserError> {
  return ok({
    yes: resolveYes(flags),
    scope: resolveScope(config, flags),
    also: config.defaults.also,
    scanMode: resolveScanMode(config, flags),
    onWarn: resolveOnWarn(config, flags),
    skipScan: flags.skipScan === true,
    trusted: false,
  });
}

export function composePolicyForSource(
  config: Config,
  flags: CliFlags,
  source: SourceForPolicy,
): Result<EffectivePolicy, UserError> {
  const base = composePolicy(config, flags);
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
