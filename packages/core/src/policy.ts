import type { Config } from "./schemas/config";
import { type Result, UserError, err, ok } from "./types";

export type CliFlags = {
  strict?: boolean;
  noStrict?: boolean;
  skipScan?: boolean;
  yes?: boolean;
  semantic?: boolean;
  project?: boolean;
  global?: boolean;
};

export type EffectivePolicy = {
  yes: boolean;
  onWarn: "prompt" | "fail";
  requireScan: boolean;
  skipScan: boolean;
  scanMode: "static" | "semantic" | "off";
  scope: "global" | "project" | "";
  also: string[];
  agentMode: boolean;
};

/**
 * Compose effective security policy from config + CLI flags.
 * Agent mode overrides everything. Otherwise, most restrictive wins.
 */
export function composePolicy(
  config: Config,
  flags: CliFlags,
): Result<EffectivePolicy, UserError> {
  const agentMode = config["agent-mode"].enabled;

  if (agentMode) {
    if (flags.skipScan) {
      return err(
        new UserError(
          "Agent mode requires security scanning. Cannot use --skip-scan.",
        ),
      );
    }

    let scope: "global" | "project";
    if (flags.project) {
      scope = "project";
    } else if (flags.global) {
      scope = "global";
    } else if (config["agent-mode"].scope) {
      scope = config["agent-mode"].scope;
    } else {
      return err(
        new UserError(
          "Agent mode requires a scope. Set agent-mode.scope in config or pass --project / --global.",
        ),
      );
    }

    return ok({
      yes: true,
      onWarn: "fail",
      requireScan: true,
      skipScan: false,
      scanMode: config.security.scan === "off" ? "static" : config.security.scan,
      scope,
      also: config.defaults.also,
      agentMode: true,
    });
  }

  // Normal mode
  if (flags.skipScan && config.security.require_scan) {
    return err(
      new UserError(
        "Security scanning is required by config (security.require_scan = true). Cannot use --skip-scan.",
      ),
    );
  }

  let onWarn: "prompt" | "fail";
  if (flags.strict) {
    onWarn = "fail";
  } else if (flags.noStrict) {
    onWarn = "prompt";
  } else {
    onWarn = config.security.on_warn;
  }

  let scope: "global" | "project" | "" = "";
  if (flags.project) {
    scope = "project";
  } else if (flags.global) {
    scope = "global";
  } else if (config.defaults.scope) {
    scope = config.defaults.scope as "global" | "project";
  }

  const scanMode =
    flags.semantic && config.security.scan !== "semantic"
      ? "semantic"
      : config.security.scan;

  return ok({
    yes: flags.yes || config.defaults.yes,
    onWarn,
    requireScan: config.security.require_scan,
    skipScan: flags.skipScan ?? false,
    scanMode,
    scope,
    also: config.defaults.also,
    agentMode: false,
  });
}
