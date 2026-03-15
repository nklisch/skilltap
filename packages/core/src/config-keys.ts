import { AUTO_UPDATE_MODES, SCOPE_VALUES, SHOW_DIFF_MODES } from "./schemas/config";
import type { Config } from "./schemas/config";
import { err, ok, type Result, UserError } from "./types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type KeyType = "string" | "boolean" | "number" | "string[]" | "enum";

export type SettableKeyDef = {
  type: KeyType;
  enum?: readonly string[];
};

// ---------------------------------------------------------------------------
// Allowlist / blocklist
// ---------------------------------------------------------------------------

export const SETTABLE_KEYS: Record<string, SettableKeyDef> = {
  "defaults.scope": { type: "enum", enum: SCOPE_VALUES },
  "defaults.also": { type: "string[]" },
  "defaults.yes": { type: "boolean" },
  "security.agent_cli": { type: "string" },
  "security.ollama_model": { type: "string" },
  "security.threshold": { type: "number" },
  "security.max_size": { type: "number" },
  "updates.auto_update": { type: "enum", enum: AUTO_UPDATE_MODES },
  "updates.interval_hours": { type: "number" },
  "updates.show_diff": { type: "enum", enum: SHOW_DIFF_MODES },
  default_git_host: { type: "string" },
};

const BLOCKED_SET_KEYS: Record<string, string> = {
  "agent-mode.enabled": "Use 'skilltap config agent-mode'",
  "agent-mode.scope": "Use 'skilltap config agent-mode'",
  "telemetry.enabled": "Use 'skilltap config telemetry enable/disable'",
  "telemetry.notice_shown": "Internal field",
  "telemetry.anonymous_id": "Internal field",
  // New per-mode security fields — use config security wizard
  "security.human.scan": "Use 'skilltap config security'",
  "security.human.on_warn": "Use 'skilltap config security'",
  "security.human.require_scan": "Use 'skilltap config security'",
  "security.agent.scan": "Use 'skilltap config security'",
  "security.agent.on_warn": "Use 'skilltap config security'",
  "security.agent.require_scan": "Use 'skilltap config security'",
  "security.overrides": "Use 'skilltap config security --trust'",
  // Old v1 keys — block with migration hint
  "security.scan": "Use 'skilltap config security' (field moved to security.human.scan / security.agent.scan)",
  "security.on_warn": "Use 'skilltap config security' (field moved to security.human.on_warn / security.agent.on_warn)",
  "security.require_scan": "Use 'skilltap config security' (field moved to security.human.require_scan / security.agent.require_scan)",
  "security.agent": "Use 'skilltap config set security.agent_cli' (field renamed to security.agent_cli)",
  taps: "Use 'skilltap tap add/remove'",
};

// ---------------------------------------------------------------------------
// Get
// ---------------------------------------------------------------------------

export function getConfigValue(
  config: Config,
  key: string,
): Result<unknown, UserError> {
  const parts = key.split(".");
  // biome-ignore lint/suspicious/noExplicitAny: walking an unknown object shape
  let current: any = config;

  for (const part of parts) {
    if (current == null || typeof current !== "object" || !(part in current)) {
      return err(
        new UserError(
          `Unknown config key: '${key}'`,
          "Run 'skilltap config get --json' to see all keys",
        ),
      );
    }
    current = current[part];
  }

  return ok(current);
}

// ---------------------------------------------------------------------------
// Validate set key
// ---------------------------------------------------------------------------

export function validateSetKey(
  key: string,
): Result<SettableKeyDef, UserError> {
  const blocked = BLOCKED_SET_KEYS[key];
  if (blocked) {
    return err(
      new UserError(`'${key}' cannot be set via 'config set'`, blocked),
    );
  }

  const def = SETTABLE_KEYS[key];
  if (!def) {
    return err(
      new UserError(
        `Unknown or non-settable key: '${key}'`,
        `Settable keys: ${Object.keys(SETTABLE_KEYS).join(", ")}`,
      ),
    );
  }

  return ok(def);
}

// ---------------------------------------------------------------------------
// Coerce string values to typed values
// ---------------------------------------------------------------------------

const TRUE_VALUES = new Set(["true", "1", "yes"]);
const FALSE_VALUES = new Set(["false", "0", "no"]);

export function coerceValue(
  rawValues: string[],
  def: SettableKeyDef,
): Result<unknown, UserError> {
  if (def.type === "string[]") {
    return ok(rawValues);
  }

  if (rawValues.length === 0) {
    return err(new UserError("Missing value"));
  }

  if (rawValues.length > 1) {
    return err(
      new UserError(
        `Expected a single value for ${def.type}, got ${rawValues.length}`,
      ),
    );
  }

  const raw = rawValues[0];

  switch (def.type) {
    case "string":
      return ok(raw);

    case "boolean": {
      const lower = raw.toLowerCase();
      if (TRUE_VALUES.has(lower)) return ok(true);
      if (FALSE_VALUES.has(lower)) return ok(false);
      return err(
        new UserError(
          `Invalid boolean: '${raw}'`,
          "Accepted values: true, false, yes, no, 1, 0",
        ),
      );
    }

    case "number": {
      const n = Number(raw);
      if (!Number.isFinite(n) || !Number.isInteger(n)) {
        return err(
          new UserError(`Invalid integer: '${raw}'`),
        );
      }
      return ok(n);
    }

    case "enum": {
      if (!def.enum?.includes(raw)) {
        return err(
          new UserError(
            `Invalid value: '${raw}'`,
            `Accepted values: ${def.enum?.map((v) => (v === "" ? '""' : v)).join(", ")}`,
          ),
        );
      }
      return ok(raw);
    }
  }
}

// ---------------------------------------------------------------------------
// Set (immutable)
// ---------------------------------------------------------------------------

export function setConfigValue(
  config: Config,
  key: string,
  value: unknown,
): Config {
  const dotIdx = key.indexOf(".");
  if (dotIdx === -1) {
    // Top-level key (no dot)
    return { ...config, [key]: value };
  }
  const section = key.slice(0, dotIdx);
  const field = key.slice(dotIdx + 1);
  // biome-ignore lint/suspicious/noExplicitAny: building a dynamic config update
  const sectionObj = { ...(config as any)[section], [field]: value };
  return { ...config, [section]: sectionObj };
}

// ---------------------------------------------------------------------------
// Format for plain-text output
// ---------------------------------------------------------------------------

export function formatConfigValue(value: unknown): string {
  if (Array.isArray(value)) {
    if (value.length > 0 && typeof value[0] === "object") {
      return `[${value.length} ${value.length === 1 ? "entry" : "entries"}]`;
    }
    return value.join(" ");
  }
  if (value == null) return "";
  return String(value);
}
