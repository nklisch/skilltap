import type { Config, TrustOverride } from "./schemas/config";
import { PRESET_VALUES, type SECURITY_PRESETS } from "./schemas/config";
import { err, ok, type Result, UserError } from "./types";

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

/**
 * Compose effective security policy from config + CLI flags.
 */
export function composePolicy(
  config: Config,
  flags: CliFlags,
): Result<EffectivePolicy, UserError> {
  const sec = config.security;

  if (flags.skipScan && sec.require_scan) {
    return err(
      new UserError(
        "Security scanning is required by config (security.require_scan = true). Cannot use --skip-scan.",
      ),
    );
  }

  let onWarn: "prompt" | "fail" | "allow";
  if (flags.strict) onWarn = "fail";
  else if (flags.noStrict) onWarn = "prompt";
  else onWarn = sec.on_warn;

  const scope = buildScope(
    flags,
    config.defaults.scope as "global" | "project" | "",
  );

  const scanMode =
    flags.semantic && sec.scan !== "semantic" ? "semantic" : sec.scan;

  return ok({
    yes: flags.yes || config.defaults.yes,
    onWarn,
    requireScan: sec.require_scan,
    skipScan: flags.skipScan ?? false,
    scanMode,
    scope,
    also: config.defaults.also,
  });
}

/**
 * Compose effective policy with a trust-tier override applied.
 * Called per-source during install when the source is known.
 * Override preset values replace config defaults; CLI flags still override on top.
 */
export function composePolicyForSource(
  config: Config,
  flags: CliFlags,
  source: { tapName?: string; sourceType: "tap" | "git" | "npm" | "local" },
): Result<EffectivePolicy, UserError> {
  const preset = resolveOverride(config.security.overrides, source);
  if (preset === null) return composePolicy(config, flags);

  const presetValues = PRESET_VALUES[preset];
  const patchedConfig: Config = {
    ...config,
    security: {
      ...config.security,
      scan: presetValues.scan,
      on_warn: presetValues.on_warn,
      require_scan: presetValues.require_scan,
    },
  };
  return composePolicy(patchedConfig, flags);
}
