import { loadConfig, saveConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine } from "../ui/format";
import { isTelemetryEnabled } from "../telemetry";

const status = defineCommand({
  meta: {
    name: "status",
    description: "Show telemetry status",
  },
  async run() {
    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    const doNotTrack = process.env.DO_NOT_TRACK === "1";
    const disabled = process.env.SKILLTAP_TELEMETRY_DISABLED === "1";

    if (doNotTrack || disabled) {
      const envVar = doNotTrack ? "DO_NOT_TRACK" : "SKILLTAP_TELEMETRY_DISABLED";
      process.stdout.write(`Telemetry: disabled (${envVar}=1 overrides config)\n`);
      return;
    }

    if (isTelemetryEnabled(config)) {
      process.stdout.write(`Telemetry: enabled\n`);
      process.stdout.write(`Anonymous ID: ${config.telemetry.anonymous_id}\n`);
    } else {
      process.stdout.write(`Telemetry: disabled\n`);
      process.stdout.write(`Run 'skilltap telemetry enable' to opt in.\n`);
    }

    process.stdout.write(`\nWhat's collected: OS, arch, CLI version, command success/failure,\n`);
    process.stdout.write(`error type, skill count, duration. No skill names, paths, or personal info.\n`);
    process.stdout.write(`Set DO_NOT_TRACK=1 or SKILLTAP_TELEMETRY_DISABLED=1 to always opt out.\n`);
  },
});

const enable = defineCommand({
  meta: {
    name: "enable",
    description: "Opt in to anonymous telemetry",
  },
  async run() {
    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    const anonymousId = config.telemetry.anonymous_id || crypto.randomUUID();
    const updated = {
      ...config,
      telemetry: { ...config.telemetry, enabled: true, anonymous_id: anonymousId },
    };

    const saveResult = await saveConfig(updated);
    if (!saveResult.ok) {
      errorLine(saveResult.error.message);
      process.exit(1);
    }

    process.stdout.write(`Telemetry enabled. Anonymous ID: ${anonymousId}\n`);
    process.stdout.write(`Run 'skilltap telemetry disable' to opt out at any time.\n`);
  },
});

const disable = defineCommand({
  meta: {
    name: "disable",
    description: "Opt out of telemetry",
  },
  async run() {
    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    const updated = {
      ...config,
      telemetry: { ...config.telemetry, enabled: false },
    };

    const saveResult = await saveConfig(updated);
    if (!saveResult.ok) {
      errorLine(saveResult.error.message);
      process.exit(1);
    }

    process.stdout.write(`Telemetry disabled.\n`);
  },
});

export default defineCommand({
  meta: {
    name: "telemetry",
    description: "Manage anonymous usage telemetry",
  },
  subCommands: { status, enable, disable },
});
