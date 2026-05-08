import { loadConfig, saveConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { isTelemetryEnabled } from "../../telemetry";
import { setupOutput } from "../../ui/setup";

const status = defineCommand({
  meta: {
    name: "skilltap config telemetry status",
    description: "Show telemetry status",
  },
  async run() {
    const out = setupOutput({ json: false, quiet: false });

    const configResult = await loadConfig();
    if (!configResult.ok) {
      out.error(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    const doNotTrack = process.env.DO_NOT_TRACK === "1";
    const disabled = process.env.SKILLTAP_TELEMETRY_DISABLED === "1";

    if (doNotTrack || disabled) {
      const envVar = doNotTrack
        ? "DO_NOT_TRACK"
        : "SKILLTAP_TELEMETRY_DISABLED";
      out.info(`Telemetry: disabled (${envVar}=1 overrides config)`);
      return;
    }

    if (isTelemetryEnabled(config)) {
      out.info(`Telemetry: enabled`);
      out.info(`Anonymous ID: ${config.telemetry.anonymous_id}`);
    } else {
      out.info(`Telemetry: disabled`);
      out.info(`Run 'skilltap config telemetry enable' to opt in.`);
    }

    out.info(`\nWhat's collected: OS, arch, CLI version, command success/failure,`);
    out.info(`error type, skill count, duration. No skill names, paths, or personal info.`);
    out.info(`Set DO_NOT_TRACK=1 or SKILLTAP_TELEMETRY_DISABLED=1 to always opt out.`);
  },
});

const enable = defineCommand({
  meta: {
    name: "skilltap config telemetry enable",
    description: "Opt in to anonymous telemetry",
  },
  async run() {
    const out = setupOutput({ json: false, quiet: false });

    const configResult = await loadConfig();
    if (!configResult.ok) {
      out.error(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    const anonymousId = config.telemetry.anonymous_id || crypto.randomUUID();
    const updated = {
      ...config,
      telemetry: {
        ...config.telemetry,
        enabled: true,
        anonymous_id: anonymousId,
      },
    };

    const saveResult = await saveConfig(updated);
    if (!saveResult.ok) {
      out.error(saveResult.error.message);
      process.exit(1);
    }

    out.info(`Telemetry enabled. Anonymous ID: ${anonymousId}`);
    out.info(`Run 'skilltap config telemetry disable' to opt out at any time.`);
  },
});

const disable = defineCommand({
  meta: {
    name: "skilltap config telemetry disable",
    description: "Opt out of telemetry",
  },
  async run() {
    const out = setupOutput({ json: false, quiet: false });

    const configResult = await loadConfig();
    if (!configResult.ok) {
      out.error(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    const updated = {
      ...config,
      telemetry: { ...config.telemetry, enabled: false },
    };

    const saveResult = await saveConfig(updated);
    if (!saveResult.ok) {
      out.error(saveResult.error.message);
      process.exit(1);
    }

    out.info(`Telemetry disabled.`);
  },
});

export default defineCommand({
  meta: {
    name: "skilltap config telemetry",
    description: "Manage anonymous usage telemetry",
  },
  subCommands: { status, enable, disable },
});
