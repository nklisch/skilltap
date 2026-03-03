import {
  coerceValue,
  loadConfig,
  saveConfig,
  setConfigValue,
  validateSetKey,
} from "@skilltap/core";
import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "set",
    description: "Set a config value",
  },
  args: {
    key: {
      type: "positional",
      description: "Config key in dot notation (e.g., defaults.scope)",
      required: true,
    },
  },
  async run({ args }) {
    const key = args.key as string;

    // Extract values from process.argv for variadic support.
    // After citty routes to "set", the positionals in argv are: key, value1, value2, ...
    const setIdx = process.argv.indexOf("set");
    const afterSet = process.argv.slice(setIdx + 1).filter((a) => !a.startsWith("-"));
    const values = afterSet.slice(1); // everything after the key

    // Validate the key is in the allowlist
    const keyResult = validateSetKey(key);
    if (!keyResult.ok) {
      process.stderr.write(`error: ${keyResult.error.message}\n`);
      if (keyResult.error.hint) process.stderr.write(`hint: ${keyResult.error.hint}\n`);
      process.exit(1);
    }

    // For non-array types, require at least one value
    if (values.length === 0 && keyResult.value.type !== "string[]") {
      process.stderr.write("error: Missing value\n");
      process.stderr.write("hint: Usage: skilltap config set <key> <value>\n");
      process.exit(1);
    }

    // Coerce string values to the target type
    const coerced = coerceValue(values, keyResult.value);
    if (!coerced.ok) {
      process.stderr.write(`error: ${coerced.error.message}\n`);
      if (coerced.error.hint) process.stderr.write(`hint: ${coerced.error.hint}\n`);
      process.exit(1);
    }

    // Load, set, save
    const configResult = await loadConfig();
    if (!configResult.ok) {
      process.stderr.write(`error: ${configResult.error.message}\n`);
      process.exit(1);
    }

    const updated = setConfigValue(configResult.value, key, coerced.value);
    const saveResult = await saveConfig(updated);
    if (!saveResult.ok) {
      process.stderr.write(`error: ${saveResult.error.message}\n`);
      process.exit(1);
    }

    // Silent on success — agent-friendly
    process.exit(0);
  },
});
