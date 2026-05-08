import { type ConfigV2, ConfigV2Schema } from "../schemas/config-v2";
import { err, ok, type Result, UserError } from "../types";

export interface RejectedHttpTap {
  name: string;
  url: string;
}

export interface ConfigMigrationResult {
  v2: ConfigV2;
  warnings: string[];
  httpTapsRejected: RejectedHttpTap[];
}

// Translate a v1.0 raw config (already parsed from TOML, before Zod) into a
// v2.0 ConfigV2. Logs lossy translations as warnings and lists HTTP taps
// separately so the orchestrator can decide to refuse migration.
//
// rawV1 may have any shape — we never trust the v1.0 schema strictly here
// since user config files in the wild may be partial.
export function migrateV1Config(
  rawV1: unknown,
): Result<ConfigMigrationResult, UserError> {
  if (rawV1 === null || rawV1 === undefined) {
    return err(new UserError("Cannot migrate: config is null or undefined"));
  }
  if (typeof rawV1 !== "object" || Array.isArray(rawV1)) {
    return err(new UserError("Cannot migrate: config is not a TOML table"));
  }
  const v1 = rawV1 as Record<string, unknown>;
  const warnings: string[] = [];

  // ── [security] ────────────────────────────────────────────────────────
  const security: { scan?: string; on_warn?: string; trust: string[] } = {
    trust: [],
  };
  const v1Sec = v1.security;
  if (v1Sec && typeof v1Sec === "object" && !Array.isArray(v1Sec)) {
    const sec = v1Sec as Record<string, unknown>;

    // Per-mode → flat. Take the stricter on conflict.
    const human = sec.human as Record<string, unknown> | undefined;
    const agent = sec.agent as Record<string, unknown> | undefined;
    const humanScan = human?.scan as string | undefined;
    const agentScan = agent?.scan as string | undefined;
    const humanOnWarn = human?.on_warn as string | undefined;
    const agentOnWarn = agent?.on_warn as string | undefined;

    if (humanScan && agentScan && humanScan !== agentScan) {
      warnings.push(
        `[security.human].scan ("${humanScan}") differs from [security.agent].scan ("${agentScan}"). ` +
          `v2.0 has one [security] block; took stricter ("${pickStricterScan(humanScan, agentScan)}").`,
      );
    }
    if (humanOnWarn && agentOnWarn && humanOnWarn !== agentOnWarn) {
      warnings.push(
        `[security.human].on_warn ("${humanOnWarn}") differs from [security.agent].on_warn ("${agentOnWarn}"). ` +
          `v2.0 has one [security] block; took stricter ("${pickStricterOnWarn(humanOnWarn, agentOnWarn)}").`,
      );
    }

    const pickedScan = pickStricterScan(humanScan, agentScan);
    if (pickedScan !== undefined)
      security.scan = mapV1Scan(pickedScan, warnings);
    const pickedOnWarn = pickStricterOnWarn(humanOnWarn, agentOnWarn);
    if (pickedOnWarn !== undefined)
      security.on_warn = mapV1OnWarn(pickedOnWarn, warnings);

    // [[security.overrides]] → security.trust (only preset = "none")
    const overrides = sec.overrides;
    if (Array.isArray(overrides)) {
      for (const ov of overrides) {
        if (ov && typeof ov === "object" && !Array.isArray(ov)) {
          const o = ov as Record<string, unknown>;
          if (o.preset === "none" && typeof o.match === "string") {
            security.trust.push(o.match);
          } else if (typeof o.match === "string") {
            warnings.push(
              `Dropped [[security.overrides]] match="${o.match}" preset="${o.preset}". ` +
                `v2.0 only supports trust-list (equivalent to preset="none"); other presets are not supported.`,
            );
          }
        }
      }
    }

    // Dropped fields: agent_cli, threshold, max_size, ollama_model
    for (const f of ["agent_cli", "threshold", "max_size", "ollama_model"]) {
      if (sec[f] !== undefined && sec[f] !== "" && sec[f] !== 0) {
        warnings.push(
          `Dropped [security].${f} (not represented in v2.0 simple model).`,
        );
      }
    }
  }

  // ── [agent] (v2.0) ←── [agent-mode] (v1.0) ──────────────────────────────
  const agentBlock: { default: boolean; block: boolean } = {
    default: false,
    block: false,
  };
  let agentModeScope: string | undefined;
  const v1AgentMode = v1["agent-mode"];
  if (
    v1AgentMode &&
    typeof v1AgentMode === "object" &&
    !Array.isArray(v1AgentMode)
  ) {
    const am = v1AgentMode as Record<string, unknown>;
    if (am.enabled === true) {
      agentBlock.default = true;
    }
    if (
      typeof am.scope === "string" &&
      (am.scope === "global" || am.scope === "project")
    ) {
      agentModeScope = am.scope;
    }
  }

  // ── [defaults] ───────────────────────────────────────────────────────────
  const defaults: { also: string[]; scope: "" | "global" | "project" } = {
    also: [],
    scope: "",
  };
  const v1Defaults = v1.defaults;
  if (
    v1Defaults &&
    typeof v1Defaults === "object" &&
    !Array.isArray(v1Defaults)
  ) {
    const d = v1Defaults as Record<string, unknown>;
    if (Array.isArray(d.also))
      defaults.also = d.also.filter((x): x is string => typeof x === "string");
    if (
      typeof d.scope === "string" &&
      (d.scope === "" || d.scope === "global" || d.scope === "project")
    ) {
      defaults.scope = d.scope;
    }
    if (d.yes === true) {
      warnings.push(
        `Dropped [defaults].yes = true. v2.0 has no global yes default; use --yes per call or [agent].default.`,
      );
    }
  }

  // Transfer [agent-mode].scope to defaults.scope if defaults.scope is not set
  if (agentModeScope && !defaults.scope) {
    defaults.scope = agentModeScope as "global" | "project";
    warnings.push(
      `[agent-mode].scope = "${agentModeScope}" transferred to [defaults].scope. ` +
        `v2.0 uses [defaults].scope for the install scope default.`,
    );
  }

  // ── [[taps]] — reject HTTP, keep git ─────────────────────────────────────
  const httpTapsRejected: RejectedHttpTap[] = [];
  const taps: { name: string; url: string }[] = [];
  const v1Taps = v1.taps;
  if (Array.isArray(v1Taps)) {
    for (const tap of v1Taps) {
      if (tap && typeof tap === "object" && !Array.isArray(tap)) {
        const t = tap as Record<string, unknown>;
        const name = typeof t.name === "string" ? t.name : null;
        const url = typeof t.url === "string" ? t.url : null;
        if (!name || !url) continue;
        if (t.type === "http") {
          httpTapsRejected.push({ name, url });
          continue;
        }
        taps.push({ name, url });
      }
    }
  }

  // ── [registry] (v1.0 — separate config block) ────────────────────────────
  // v2.0 keeps registry support but the block is unchanged structurally; we
  // just pass through the existing taps. Custom registry sources are out of
  // scope for the migration (they're a pure-additive feature; users keep
  // them by not editing).
  const v1Registry = v1.registry;
  if (v1Registry !== undefined) {
    warnings.push(
      `[registry] block was present in v1.0 config. It is preserved in v2.0 but should be reviewed manually.`,
    );
  }

  // ── pass-through blocks: [updates], [telemetry] ─────────────────────────
  const updates = (v1.updates as Record<string, unknown> | undefined) ?? {};
  const telemetry = (v1.telemetry as Record<string, unknown> | undefined) ?? {};

  // ── builtin_tap, verbose, default_git_host ───────────────────────────────
  const builtinTap =
    typeof v1.builtin_tap === "boolean" ? v1.builtin_tap : true;
  const verbose = typeof v1.verbose === "boolean" ? v1.verbose : true;
  const defaultGitHost =
    typeof v1.default_git_host === "string"
      ? v1.default_git_host
      : "https://github.com";

  // Build the v2 candidate
  const v2Candidate = {
    defaults,
    agent: agentBlock,
    security,
    taps,
    updates,
    telemetry,
    builtin_tap: builtinTap,
    verbose,
    default_git_host: defaultGitHost,
  };

  // Validate via Zod — applies defaults for any fields we left off
  const parsed = ConfigV2Schema.safeParse(v2Candidate);
  if (!parsed.success) {
    return err(
      new UserError(
        `Migrated config failed v2.0 validation: ${JSON.stringify(parsed.error.issues, null, 2)}`,
      ),
    );
  }

  return ok({
    v2: parsed.data,
    warnings,
    httpTapsRejected,
  });
}

// Order: semantic > static > none (semantic is the most paranoid).
// Returns whichever scan name is "stricter". Undefined inputs are skipped.
function pickStricterScan(
  a: string | undefined,
  b: string | undefined,
): string | undefined {
  const order: Record<string, number> = {
    off: 0,
    none: 0,
    static: 1,
    semantic: 2,
  };
  if (a === undefined && b === undefined) return undefined;
  if (a === undefined) return b;
  if (b === undefined) return a;
  return (order[a] ?? 0) >= (order[b] ?? 0) ? a : b;
}

// Order: install < prompt < fail (fail is the most blocking).
function pickStricterOnWarn(
  a: string | undefined,
  b: string | undefined,
): string | undefined {
  const order: Record<string, number> = {
    allow: 0,
    install: 0,
    prompt: 1,
    fail: 2,
  };
  if (a === undefined && b === undefined) return undefined;
  if (a === undefined) return b;
  if (b === undefined) return a;
  return (order[a] ?? 0) >= (order[b] ?? 0) ? a : b;
}

function mapV1Scan(v1: string, warnings: string[]): string {
  if (v1 === "off") return "none";
  if (v1 === "static" || v1 === "semantic" || v1 === "none") return v1;
  warnings.push(
    `Unknown v1 [security].scan value "${v1}" — defaulted to "static".`,
  );
  return "static";
}

function mapV1OnWarn(v1: string, warnings: string[]): string {
  if (v1 === "allow") return "install";
  if (v1 === "prompt" || v1 === "fail") return v1;
  if (v1 === "install") return v1;
  warnings.push(
    `Unknown v1 [security].on_warn value "${v1}" — defaulted to "install".`,
  );
  return "install";
}
