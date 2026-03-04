import {
  formatConfigValue,
  getConfigValue,
  loadConfig,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { agentError } from "../../ui/agent-out";
import { errorLine } from "../../ui/format";

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
    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;
    const key = args.key as string | undefined;

    if (!key) {
      if (args.json) {
        process.stdout.write(`${JSON.stringify(config, null, 2)}\n`);
      } else {
        printFlat(config);
      }
      process.exit(0);
    }

    const result = getConfigValue(config, key);
    if (!result.ok) {
      const isAgentMode = config["agent-mode"].enabled;
      if (isAgentMode) {
        agentError(result.error.message);
      } else {
        process.stderr.write(`error: ${result.error.message}\n`);
        if (result.error.hint) process.stderr.write(`  hint: ${result.error.hint}\n`);
      }
      process.exit(1);
    }

    if (args.json) {
      process.stdout.write(`${JSON.stringify(result.value, null, 2)}\n`);
    } else {
      process.stdout.write(`${formatConfigValue(result.value)}\n`);
    }
    process.exit(0);
  },
});

function printFlat(config: Record<string, unknown>): void {
  for (const [section, value] of Object.entries(config)) {
    if (Array.isArray(value)) {
      process.stdout.write(
        `${section} = ${formatConfigValue(value)}\n`,
      );
    } else if (value != null && typeof value === "object") {
      for (const [field, v] of Object.entries(
        value as Record<string, unknown>,
      )) {
        process.stdout.write(
          `${section}.${field} = ${formatConfigValue(v)}\n`,
        );
      }
    }
  }
}
