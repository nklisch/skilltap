import { cancel, group, intro, isCancel, note, outro } from "@clack/prompts";
import {
  footerMultiselect as multiselect,
  footerSelect as select,
} from "../../ui/footer";
import { AGENT_LABELS, loadConfig, PRESET_VALUES, SECURITY_PRESETS, saveConfig, VALID_AGENT_IDS } from "@skilltap/core";
import { SCAN_MODE_OPTIONS } from "../../ui/prompts";
import { defineCommand } from "citty";
import { errorLine } from "../../ui/format";
import { selectAgentForConfig } from "../../ui/prompts";

const PRESET_OPTIONS = [
  { value: "none", label: "None", hint: "no scanning" },
  { value: "relaxed", label: "Relaxed", hint: "static scan, ignore warnings" },
  { value: "standard", label: "Standard", hint: "static scan, ask on warnings (Recommended)" },
  { value: "strict", label: "Strict", hint: "static + semantic scan, block on warnings" },
  { value: "custom", label: "Custom", hint: "set individual options" },
];

const ON_WARN_OPTIONS = [
  { value: "prompt", label: "Ask me (prompt)" },
  { value: "fail", label: "Always block (fail)" },
  { value: "allow", label: "Ignore warnings (allow)" },
];

export default defineCommand({
  meta: {
    name: "skilltap config agent-mode",
    description: "Enable or disable agent mode (interactive only)",
  },
  async run() {
    if (!process.stdin.isTTY) {
      process.stderr.write(
        "error: 'skilltap config agent-mode' must be run interactively.\n" +
          "Agent mode can only be enabled or disabled by a human.\n",
      );
      process.exit(1);
    }

    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const config = configResult.value;

    intro("Agent Mode Setup");

    note(
      "Agent mode changes how skilltap behaves when called by AI agents:\n" +
        "  - All prompts auto-accept or hard-fail (no interactive input)\n" +
        "  - Output is plain text (no colors or spinners)\n" +
        "  - Security behavior is governed by the agent security profile",
      "What is agent mode?",
    );

    const enableResult = await select({
      message: "Enable agent mode?",
      options: [
        { value: true, label: "Yes" },
        { value: false, label: "No (disable)" },
      ],
      initialValue: config["agent-mode"].enabled,
    });
    if (isCancel(enableResult)) {
      cancel("Cancelled.");
      process.exit(130);
    }

    const enabled = enableResult as boolean;

    if (!enabled) {
      config["agent-mode"].enabled = false;
      const saveResult = await saveConfig(config);
      if (!saveResult.ok) {
        errorLine(saveResult.error.message);
        process.exit(1);
      }
      outro("Agent mode disabled");
      return;
    }

    const settings = await group(
      {
        scope: () =>
          select({
            message: "Default scope for agent installs?",
            options: [
              {
                value: "project",
                label: "Project",
                hint: "recommended — agents work in project context",
              },
              { value: "global", label: "Global" },
            ],
            initialValue: config["agent-mode"].scope,
          }),

        also: () =>
          multiselect({
            message: "Auto-symlink to which agents?",
            options: VALID_AGENT_IDS.map(id => ({ value: id, label: AGENT_LABELS[id] ?? id })),
            initialValues: config.defaults.also,
            required: false,
          }),

        preset: () =>
          select({
            message: "Security preset for agent mode?",
            options: PRESET_OPTIONS,
            initialValue: "strict",
          }),

        scan: ({ results }) => {
          if (results.preset !== "custom") return Promise.resolve(undefined);
          return select({
            message: "Scan level for agent installs?",
            options: SCAN_MODE_OPTIONS,
            initialValue: config.security.agent.scan,
          });
        },

        onWarn: ({ results }) => {
          if (results.preset !== "custom") return Promise.resolve(undefined);
          return select({
            message: "When warnings are found?",
            options: ON_WARN_OPTIONS,
            initialValue: config.security.agent.on_warn,
          });
        },

        requireScan: ({ results }) => {
          if (results.preset !== "custom") return Promise.resolve(undefined);
          return select({
            message: "Require scanning? (block --skip-scan)",
            options: [
              { value: true, label: "Yes" },
              { value: false, label: "No" },
            ],
            initialValue: config.security.agent.require_scan,
          });
        },

        agentCli: ({ results }) => {
          const needsSemantic =
            results.preset === "strict" ||
            (results.preset === "custom" && results.scan === "semantic");
          if (!needsSemantic) return Promise.resolve(config.security.agent_cli);
          return selectAgentForConfig(config.security.agent_cli);
        },
      },
      {
        onCancel() {
          cancel("Cancelled.");
          process.exit(130);
        },
      },
    );

    config["agent-mode"] = {
      enabled: true,
      scope: settings.scope as "global" | "project",
    };
    config.defaults.also = settings.also as string[];

    if (settings.preset !== "custom") {
      const preset = settings.preset as (typeof SECURITY_PRESETS)[number];
      config.security.agent = { ...PRESET_VALUES[preset] };
    } else {
      config.security.agent = {
        scan: settings.scan as "static" | "semantic" | "off",
        on_warn: settings.onWarn as "prompt" | "fail" | "allow",
        require_scan: settings.requireScan as boolean,
      };
    }

    if (settings.agentCli) config.security.agent_cli = settings.agentCli as string;

    const saveResult = await saveConfig(config);
    if (!saveResult.ok) {
      errorLine(saveResult.error.message);
      process.exit(1);
    }

    const preset = settings.preset as string;
    const scanLabel = preset !== "custom"
      ? preset
      : `${settings.scan as string}`;
    outro(
      `Agent mode enabled\n  Scope: ${settings.scope as string}\n  Security: ${scanLabel}`,
    );
  },
});
