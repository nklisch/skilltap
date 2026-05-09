import { type Config, ConfigSchema } from "../schemas/config";
import { err, ok, type Result, UserError } from "../types";

export interface RejectedHttpTap {
  name: string;
  url: string;
}

export interface ConfigMigrationResult {
  migrated: Config;
  warnings: string[];
  httpTapsRejected: RejectedHttpTap[];
}

// Translate a v0.x or legacy-v2.x raw config (already parsed from TOML, before
// Zod) into a V2 Config. Logs lossy translations as warnings and lists HTTP
// taps separately so the orchestrator can decide to refuse migration.
//
// raw may have any shape — we never trust the legacy schema strictly here
// since user config files in the wild may be partial.
export function migrateV1Config(
  raw: unknown,
): Result<ConfigMigrationResult, UserError> {
  if (raw === null || raw === undefined) {
    return err(new UserError("Cannot migrate: config is null or undefined"));
  }
  if (typeof raw !== "object" || Array.isArray(raw)) {
    return err(new UserError("Cannot migrate: config is not a TOML table"));
  }
  const v1 = raw as Record<string, unknown>;
  const warnings: string[] = [];

  // ── [security] (V2: scan, on_warn, trust) ──────────────────────────────
  const security: { scan?: string; on_warn?: string; trust: string[] } = {
    trust: [],
  };
  // ── [scanner] (V2: agent_cli, ollama_model, threshold, max_size) ───────
  const scanner: {
    agent_cli?: string;
    ollama_model?: string;
    threshold?: number;
    max_size?: number;
  } = {};

  const v1Sec = v1.security;
  if (v1Sec && typeof v1Sec === "object" && !Array.isArray(v1Sec)) {
    const sec = v1Sec as Record<string, unknown>;

    // Per-mode (v0.x) → flat. Take the stricter on conflict.
    const human = sec.human as Record<string, unknown> | undefined;
    const agent = sec.agent as Record<string, unknown> | undefined;
    const humanScan = human?.scan as string | undefined;
    const agentScan = agent?.scan as string | undefined;
    const humanOnWarn = human?.on_warn as string | undefined;
    const agentOnWarn = agent?.on_warn as string | undefined;

    if (humanScan && agentScan && humanScan !== agentScan) {
      warnings.push(
        `[security.human].scan ("${humanScan}") differs from [security.agent].scan ("${agentScan}"). ` +
          `v2.2 has one [security] block; took stricter ("${pickStricterScan(humanScan, agentScan)}").`,
      );
    }
    if (humanOnWarn && agentOnWarn && humanOnWarn !== agentOnWarn) {
      warnings.push(
        `[security.human].on_warn ("${humanOnWarn}") differs from [security.agent].on_warn ("${agentOnWarn}"). ` +
          `v2.2 has one [security] block; took stricter ("${pickStricterOnWarn(humanOnWarn, agentOnWarn)}").`,
      );
    }

    // Top-level [security].scan / on_warn (legacy-v2.x flat shape) — also
    // considered, with per-mode taking precedence if both present.
    const topScan = typeof sec.scan === "string" ? sec.scan : undefined;
    const topOnWarn = typeof sec.on_warn === "string" ? sec.on_warn : undefined;

    const pickedScan =
      pickStricterScan(humanScan, agentScan) ??
      (topScan !== undefined ? topScan : undefined);
    if (pickedScan !== undefined)
      security.scan = mapV1Scan(pickedScan, warnings);
    const pickedOnWarn =
      pickStricterOnWarn(humanOnWarn, agentOnWarn) ??
      (topOnWarn !== undefined ? topOnWarn : undefined);
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
                `preset \`${o.preset}\` removed in v2.2; reconfigure with explicit \`scan\`/\`on_warn\` if needed.`,
            );
          }
        }
      }
    }

    // require_scan removed in v2.2 — both flat and per-mode forms.
    const dropRequireScan = (where: string, value: unknown) => {
      if (value !== undefined) {
        warnings.push(
          `Dropped ${where}.require_scan = ${JSON.stringify(value)}. ` +
            `require_scan removed; set \`on_warn = 'fail'\` if you want hard-fail behavior.`,
        );
      }
    };
    dropRequireScan("[security]", sec.require_scan);
    if (human) dropRequireScan("[security.human]", human.require_scan);
    if (agent) dropRequireScan("[security.agent]", agent.require_scan);

    // Operational keys → [scanner]. Sources, in priority:
    //   1. flat [security].agent_cli / threshold / max_size / ollama_model (legacy-v2.x)
    //   2. [security.human].agent_cli etc. (v0.x)
    //   3. [security.agent].agent_cli etc. (v0.x — fallback)
    const pickOpString = (
      key: "agent_cli" | "ollama_model",
    ): string | undefined => {
      const candidates: unknown[] = [sec[key], human?.[key], agent?.[key]];
      for (const c of candidates) {
        if (typeof c === "string" && c !== "") return c;
      }
      return undefined;
    };
    const pickOpNumber = (
      key: "threshold" | "max_size",
    ): number | undefined => {
      const candidates: unknown[] = [sec[key], human?.[key], agent?.[key]];
      for (const c of candidates) {
        if (typeof c === "number") return c;
      }
      return undefined;
    };
    const agentCli = pickOpString("agent_cli");
    if (agentCli !== undefined) scanner.agent_cli = agentCli;
    const ollamaModel = pickOpString("ollama_model");
    if (ollamaModel !== undefined) scanner.ollama_model = ollamaModel;
    const threshold = pickOpNumber("threshold");
    if (threshold !== undefined) scanner.threshold = threshold;
    const maxSize = pickOpNumber("max_size");
    if (maxSize !== undefined) scanner.max_size = maxSize;

    // Surface that we translated these (so users know where they went).
    for (const f of ["agent_cli", "threshold", "max_size", "ollama_model"]) {
      const present =
        sec[f] !== undefined && sec[f] !== "" && sec[f] !== 0
          ? "[security]"
          : human?.[f] !== undefined && human[f] !== "" && human[f] !== 0
            ? "[security.human]"
            : agent?.[f] !== undefined && agent[f] !== "" && agent[f] !== 0
              ? "[security.agent]"
              : null;
      if (present) {
        warnings.push(
          `Translated ${present}.${f} → [scanner].${f} (operational scanner config moved to its own block in v2.2).`,
        );
      }
    }
  }

  // ── [agent-mode] (v0.x) — dropped in v2.2. ─────────────────────────────
  // We still salvage [agent-mode].scope into defaults.scope when defaults
  // doesn't already pin one.
  let agentModeScope: string | undefined;
  const v1AgentMode = v1["agent-mode"];
  if (
    v1AgentMode &&
    typeof v1AgentMode === "object" &&
    !Array.isArray(v1AgentMode)
  ) {
    const am = v1AgentMode as Record<string, unknown>;
    if (
      typeof am.scope === "string" &&
      (am.scope === "global" || am.scope === "project")
    ) {
      agentModeScope = am.scope;
    }
    warnings.push(
      `Dropped [agent-mode] block. Agent-mode behavior is removed in v2.2; ` +
        `non-interactive runs are driven by --yes / --json / TTY detection.`,
    );
  }

  // ── [agent] (legacy-v2.x) — also dropped in v2.2. ──────────────────────
  if (v1.agent && typeof v1.agent === "object" && !Array.isArray(v1.agent)) {
    warnings.push(
      `Dropped [agent] block. Agent-mode config is removed in v2.2; ` +
        `non-interactive runs are driven by --yes / --json / TTY detection.`,
    );
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
        `Dropped [defaults].yes = true. v2.2 has no global yes default; use --yes per call.`,
      );
    }
  }

  // Transfer [agent-mode].scope to defaults.scope if defaults.scope is not set
  if (agentModeScope && !defaults.scope) {
    defaults.scope = agentModeScope as "global" | "project";
    warnings.push(
      `[agent-mode].scope = "${agentModeScope}" transferred to [defaults].scope. ` +
        `v2.2 uses [defaults].scope for the install scope default.`,
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

  // ── [registry] — preserve enabled/sources, drop allow_npm silently. ─────
  const registry: Record<string, unknown> = {};
  const v1Registry = v1.registry;
  if (
    v1Registry &&
    typeof v1Registry === "object" &&
    !Array.isArray(v1Registry)
  ) {
    const r = v1Registry as Record<string, unknown>;
    if (Array.isArray(r.enabled)) registry.enabled = r.enabled;
    if (Array.isArray(r.sources)) registry.sources = r.sources;
    // allow_npm: dropped silently per design.
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

  // Build the V2 candidate
  const v2Candidate: Record<string, unknown> = {
    defaults,
    security,
    scanner,
    registry,
    taps,
    updates,
    telemetry,
    builtin_tap: builtinTap,
    verbose,
    default_git_host: defaultGitHost,
  };

  // Validate via Zod — applies defaults for any fields we left off
  const parsed = ConfigSchema.safeParse(v2Candidate);
  if (!parsed.success) {
    return err(
      new UserError(
        `Migrated config failed v2.2 validation: ${JSON.stringify(parsed.error.issues, null, 2)}`,
      ),
    );
  }

  return ok({
    migrated: parsed.data,
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
    `Unknown legacy [security].scan value "${v1}" — defaulted to "static".`,
  );
  return "static";
}

function mapV1OnWarn(v1: string, warnings: string[]): string {
  if (v1 === "allow") return "install";
  if (v1 === "prompt" || v1 === "fail") return v1;
  if (v1 === "install") return v1;
  warnings.push(
    `Unknown legacy [security].on_warn value "${v1}" — defaulted to "install".`,
  );
  return "install";
}
