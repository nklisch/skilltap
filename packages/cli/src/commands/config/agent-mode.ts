import {
  cancel,
  group,
  intro,
  isCancel,
  multiselect,
  note,
  outro,
  select,
} from "@clack/prompts";
import { loadConfig, saveConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine } from "../../ui/format";
import { selectAgentForConfig } from "../../ui/prompts";

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
        "  - Security warnings always block installation\n" +
        "  - Security scanning cannot be skipped\n" +
        "  - Output is plain text (no colors or spinners)",
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
      process.exit(2);
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
            options: [
              { value: "claude-code", label: "Claude Code" },
              { value: "cursor", label: "Cursor" },
              { value: "codex", label: "Codex" },
              { value: "gemini", label: "Gemini" },
              { value: "windsurf", label: "Windsurf" },
            ],
            initialValues: config.defaults.also,
            required: false,
          }),

        scan: () =>
          select({
            message: "Security scan level for agent installs?",
            options: [
              { value: "static", label: "Static only", hint: "fast" },
              {
                value: "semantic",
                label: "Static + Semantic",
                hint: "thorough",
              },
            ],
            initialValue:
              config.security.scan === "off" ? "static" : config.security.scan,
          }),

        agent: ({ results }) => {
          if (results.scan !== "semantic")
            return Promise.resolve(config.security.agent);
          return selectAgentForConfig(config.security.agent);
        },
      },
      {
        onCancel() {
          cancel("Cancelled.");
          process.exit(2);
        },
      },
    );

    config["agent-mode"] = {
      enabled: true,
      scope: settings.scope as "global" | "project",
    };
    config.defaults.also = settings.also as string[];
    config.security.scan = settings.scan as "static" | "semantic";
    if (settings.agent) config.security.agent = settings.agent as string;

    const saveResult = await saveConfig(config);
    if (!saveResult.ok) {
      errorLine(saveResult.error.message);
      process.exit(1);
    }

    const scanLabel =
      settings.scan === "semantic" ? "static + semantic" : "static";
    outro(
      `Agent mode enabled\n  Scope: ${settings.scope}\n  Security: ${scanLabel}, strict`,
    );
  },
});
