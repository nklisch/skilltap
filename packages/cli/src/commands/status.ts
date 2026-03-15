import { BUILTIN_TAP, describeSecurityMode, loadConfig } from "@skilltap/core";
import { defineCommand } from "citty";

export default defineCommand({
  meta: {
    name: "status",
    description: "Show agent mode status and configuration",
  },
  args: {
    json: {
      type: "boolean",
      description: "Output as JSON",
      default: false,
    },
  },
  async run({ args }) {
    const result = await loadConfig();
    if (!result.ok) {
      process.stderr.write(`ERROR: ${result.error.message}\n`);
      process.exit(1);
    }

    const config = result.value;
    const agentMode = config["agent-mode"];
    const security = config.security;
    const defaults = config.defaults;
    const hasBuiltin = config.builtin_tap !== false;
    const tapCount = config.taps.length + (hasBuiltin ? 1 : 0);

    if (args.json) {
      process.stdout.write(
        `${JSON.stringify(
          {
            agentMode: agentMode.enabled,
            scope: agentMode.enabled
              ? agentMode.scope
              : (defaults.scope || null),
            security: {
              human: security.human,
              agent: security.agent,
              agent_cli: security.agent_cli || null,
            },
            also: defaults.also,
            taps: tapCount,
          },
          null,
          2,
        )}\n`,
      );
      return;
    }

    const scope = agentMode.enabled
      ? agentMode.scope
      : (defaults.scope || "(not configured)");

    process.stdout.write(
      [
        `agent-mode: ${agentMode.enabled ? "enabled" : "disabled"}`,
        `scope: ${scope}`,
        `security.human: ${describeSecurityMode(security.human)}`,
        `security.agent: ${describeSecurityMode(security.agent)}`,
        `agent_cli: ${security.agent_cli || "(none)"}`,
        `also: ${defaults.also.length > 0 ? defaults.also.join(" ") : "(none)"}`,
        `taps: ${tapCount}`,
      ].join("\n") + "\n",
    );
  },
});
