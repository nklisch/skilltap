import type { Config, TrustOverride } from "./schemas/config";
import { PRESET_VALUES, SECURITY_PRESETS } from "./schemas/config";
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
  onWarn: "prompt" | "fail" | "allow";
  requireScan: boolean;
  skipScan: boolean;
  scanMode: "static" | "semantic" | "off";
  scope: "global" | "project" | "";
  also: string[];
  agentMode: boolean;
};

/**
 * Map a source adapter name to the canonical source type for override matching.
 * "github" and "http" are git-hosted sources and map to "git".
 */
export function mapAdapterToSourceType(
  adapter: string,
): "tap" | "git" | "npm" | "local" {
  switch (adapter) {
    case "npm":
      return "npm";
    case "local":
      return "local";
    case "tap":
      return "tap";
    default:
      // "git", "github", "http" and anything else map to "git"
      return "git";
  }
}

/**
 * Resolve trust-tier override for a given install source.
 * Named tap match (exact) takes priority over source type match.
 * Returns the preset name if a matching override exists, null otherwise.
 */
export function resolveOverride(
  overrides: TrustOverride[],
  source: { tapName?: string; sourceType: "tap" | "git" | "npm" | "local" },
): (typeof SECURITY_PRESETS)[number] | null {
  // First pass: named tap match
  if (source.tapName) {
    for (const o of overrides) {
      if (o.kind === "tap" && o.match === source.tapName) {
        return o.preset;
      }
    }
  }

  // Second pass: source type match
  for (const o of overrides) {
    if (o.kind === "source" && o.match === source.sourceType) {
      return o.preset;
    }
  }

  return null;
}

function buildScope(
  flags: CliFlags,
  configScope: "global" | "project" | "",
): "global" | "project" | "" {
  if (flags.project) return "project";
  if (flags.global) return "global";
  return configScope;
}

function buildAgentScope(
  flags: CliFlags,
  agentModeScope: "global" | "project",
): Result<"global" | "project", UserError> {
  if (flags.project) return ok("project");
  if (flags.global) return ok("global");
  if (agentModeScope) return ok(agentModeScope);
  return err(
    new UserError(
      "Agent mode requires a scope. Set agent-mode.scope in config or pass --project / --global.",
    ),
  );
}

/**
 * Compose effective security policy from config + CLI flags.
 * Uses per-mode settings (human vs agent). No enforced minimums on agent mode.
 */
export function composePolicy(
  config: Config,
  flags: CliFlags,
): Result<EffectivePolicy, UserError> {
  const agentMode = config["agent-mode"].enabled;

  if (agentMode) {
    if (flags.skipScan && config.security.agent.require_scan) {
      return err(
        new UserError(
          "Agent mode requires security scanning. Cannot use --skip-scan.",
        ),
      );
    }

    const scopeResult = buildAgentScope(flags, config["agent-mode"].scope);
    if (!scopeResult.ok) return scopeResult;

    const agentSec = config.security.agent;

    let onWarn: "prompt" | "fail" | "allow";
    if (flags.strict) {
      onWarn = "fail";
    } else if (flags.noStrict) {
      onWarn = "prompt";
    } else {
      onWarn = agentSec.on_warn;
    }

    const scanMode =
      flags.semantic && agentSec.scan !== "semantic"
        ? "semantic"
        : agentSec.scan;

    return ok({
      yes: true,
      onWarn,
      requireScan: agentSec.require_scan,
      skipScan: flags.skipScan ?? false,
      scanMode,
      scope: scopeResult.value,
      also: config.defaults.also,
      agentMode: true,
    });
  }

  // Human mode
  const humanSec = config.security.human;

  if (flags.skipScan && humanSec.require_scan) {
    return err(
      new UserError(
        "Security scanning is required by config (security.require_scan = true). Cannot use --skip-scan.",
      ),
    );
  }

  let onWarn: "prompt" | "fail" | "allow";
  if (flags.strict) {
    onWarn = "fail";
  } else if (flags.noStrict) {
    onWarn = "prompt";
  } else {
    onWarn = humanSec.on_warn;
  }

  const scope = buildScope(
    flags,
    config.defaults.scope as "global" | "project" | "",
  );

  const scanMode =
    flags.semantic && humanSec.scan !== "semantic"
      ? "semantic"
      : humanSec.scan;

  return ok({
    yes: flags.yes || config.defaults.yes,
    onWarn,
    requireScan: humanSec.require_scan,
    skipScan: flags.skipScan ?? false,
    scanMode,
    scope,
    also: config.defaults.also,
    agentMode: false,
  });
}

/**
 * Compose effective policy with a trust-tier override applied.
 * Called per-source during install when the source is known.
 * Override preset values replace mode defaults; CLI flags still override on top.
 */
export function composePolicyForSource(
  config: Config,
  flags: CliFlags,
  source: { tapName?: string; sourceType: "tap" | "git" | "npm" | "local" },
): Result<EffectivePolicy, UserError> {
  const preset = resolveOverride(config.security.overrides, source);
  if (preset === null) {
    return composePolicy(config, flags);
  }

  // Apply preset values as mode overrides, then recompose
  const presetValues = PRESET_VALUES[preset];
  const agentMode = config["agent-mode"].enabled;
  const modeKey = agentMode ? "agent" : "human";

  const patchedConfig: Config = {
    ...config,
    security: {
      ...config.security,
      [modeKey]: {
        ...config.security[modeKey],
        scan: presetValues.scan,
        on_warn: presetValues.on_warn,
        require_scan: presetValues.require_scan,
      },
    },
  };

  return composePolicy(patchedConfig, flags);
}
