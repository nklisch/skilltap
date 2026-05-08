import type { Output } from "@skilltap/core";
import { formatConfigValue, getConfigValue, loadConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { createOutput } from "../../output";

export default defineCommand({
  meta: {
    name: "skilltap config get",
    description: "Get a config value",
  },
  args: {
    key: {
      type: "positional",
      description: "Config key in dot notation (e.g., defaults.scope)",
      required: false,
    },
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const out = createOutput({ json: args.json, quiet: false });

    const configResult = await loadConfig();
    if (!configResult.ok) {
      out.error(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;
    const key = args.key as string | undefined;

    if (!key) {
      if (args.json) {
        out.json(config);
      } else {
        printFlat(config, out);
      }
      process.exit(0);
    }

    const result = getConfigValue(config, key);
    if (!result.ok) {
      out.error(result.error.message, result.error.hint);
      process.exit(1);
    }

    if (args.json) {
      out.json(result.value);
    } else {
      out.raw(`${formatConfigValue(result.value)}\n`);
    }
    process.exit(0);
  },
});

function printFlat(config: Record<string, unknown>, out: Output): void {
  for (const [section, value] of Object.entries(config)) {
    if (Array.isArray(value)) {
      out.raw(`${section} = ${formatConfigValue(value)}\n`);
    } else if (value != null && typeof value === "object") {
      for (const [field, v] of Object.entries(
        value as Record<string, unknown>,
      )) {
        out.raw(`${section}.${field} = ${formatConfigValue(v)}\n`);
      }
    }
  }
}
