import {
  coerceValue,
  formatConfigValue,
  loadConfig,
  saveConfig,
  setConfigValue,
  validateSetKey,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../../output";

export default defineCommand({
  meta: {
    name: "skilltap config set",
    description: "Set a config value",
  },
  args: {
    key: {
      type: "positional",
      description: "Config key in dot notation (e.g., defaults.scope)",
      required: true,
    },
    value: {
      type: "positional",
      description: "New value to set",
      required: false,
    },
  },
  async run({ args }) {
    const out = createOutput({ json: false, quiet: false });
    const key = args.key as string;

    // Extract values from process.argv for variadic support.
    // After citty routes to "set", the positionals in argv are: key, value1, value2, ...
    const setIdx = process.argv.indexOf("set");
    const afterSet = process.argv
      .slice(setIdx + 1)
      .filter((a) => !a.startsWith("-"));
    const values = afterSet.slice(1); // everything after the key

    const configResult = await loadConfig();

    if (!configResult.ok) {
      out.error(configResult.error.message);
      process.exit(1);
    }

    // Validate the key is in the allowlist
    const keyResult = validateSetKey(key);
    if (!keyResult.ok) {
      out.error(keyResult.error.message, keyResult.error.hint);
      process.exit(1);
    }

    // For non-array types, require at least one value
    if (values.length === 0 && keyResult.value.type !== "string[]") {
      out.error("Missing value", "Usage: skilltap config set <key> <value>");
      process.exit(1);
    }

    // Coerce string values to the target type
    const coerced = coerceValue(values, keyResult.value);
    if (!coerced.ok) {
      out.error(coerced.error.message, coerced.error.hint);
      process.exit(1);
    }

    const updated = setConfigValue(configResult.value, key, coerced.value);
    const saveResult = await saveConfig(updated);
    if (!saveResult.ok) {
      out.error(saveResult.error.message);
      process.exit(1);
    }

    out.raw(`OK: ${key} = ${formatConfigValue(coerced.value)}\n`);
  },
});
