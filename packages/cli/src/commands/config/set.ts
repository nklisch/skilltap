import {
  coerceValue,
  formatConfigValue,
  loadConfig,
  saveConfig,
  setConfigValue,
  validateSetKey,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { agentError } from "../../ui/agent-out";

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
    const key = args.key as string;

    // Extract values from process.argv for variadic support.
    // After citty routes to "set", the positionals in argv are: key, value1, value2, ...
    const setIdx = process.argv.indexOf("set");
    const afterSet = process.argv.slice(setIdx + 1).filter((a) => !a.startsWith("-"));
    const values = afterSet.slice(1); // everything after the key

    // Load config first so we can check agent mode for error formatting
    const configResult = await loadConfig();
    const agentMode = configResult.ok && configResult.value["agent-mode"].enabled;

    const writeError = (msg: string, hint?: string) => {
      if (agentMode) {
        agentError(msg);
      } else {
        process.stderr.write(`ERROR: ${msg}\n`);
        if (hint) process.stderr.write(`  hint: ${hint}\n`);
      }
    };

    if (!configResult.ok) {
      writeError(configResult.error.message);
      process.exit(1);
    }

    // Validate the key is in the allowlist
    const keyResult = validateSetKey(key);
    if (!keyResult.ok) {
      writeError(keyResult.error.message, keyResult.error.hint);
      process.exit(1);
    }

    // For non-array types, require at least one value
    if (values.length === 0 && keyResult.value.type !== "string[]") {
      writeError("Missing value", "Usage: skilltap config set <key> <value>");
      process.exit(1);
    }

    // Coerce string values to the target type
    const coerced = coerceValue(values, keyResult.value);
    if (!coerced.ok) {
      writeError(coerced.error.message, coerced.error.hint);
      process.exit(1);
    }

    const updated = setConfigValue(configResult.value, key, coerced.value);
    const saveResult = await saveConfig(updated);
    if (!saveResult.ok) {
      writeError(saveResult.error.message);
      process.exit(1);
    }

    process.stdout.write(`OK: ${key} = ${formatConfigValue(coerced.value)}\n`);
  },
});
