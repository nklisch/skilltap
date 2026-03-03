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
  "defaults.scope": { type: "enum", enum: ["", "global", "project"] },
  "defaults.also": { type: "string[]" },
  "defaults.yes": { type: "boolean" },
  "security.agent": { type: "string" },
  "security.ollama_model": { type: "string" },
  "updates.auto_update": { type: "enum", enum: ["off", "patch", "minor"] },
  "updates.interval_hours": { type: "number" },
};

const BLOCKED_SET_KEYS: Record<string, string> = {
  "agent-mode.enabled": "Use 'skilltap config agent-mode'",
  "agent-mode.scope": "Use 'skilltap config agent-mode'",
  "telemetry.enabled": "Use 'skilltap config telemetry enable/disable'",
  "telemetry.notice_shown": "Internal field",
  "telemetry.anonymous_id": "Internal field",
  "security.scan": "Use 'skilltap config' interactive wizard",
  "security.on_warn": "Use 'skilltap config' interactive wizard",
  "security.require_scan": "Use 'skilltap config' interactive wizard",
  "security.max_size": "Use 'skilltap config' interactive wizard",
  "security.threshold": "Use 'skilltap config' interactive wizard",
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
  const [section, field] = key.split(".");
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
