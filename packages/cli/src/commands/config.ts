import {
  cancel,
  confirm,
  group,
  intro,
  isCancel,
  multiselect,
  outro,
  select,
} from "@clack/prompts";
import { type Config, getConfigDir, loadConfig, saveConfig } from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine } from "../ui/format";
import { selectAgentForConfig } from "../ui/prompts";

export default defineCommand({
  meta: {
    name: "config",
    description: "Interactive setup wizard",
  },
  args: {
    reset: {
      type: "boolean",
      description: "Overwrite existing config",
      default: false,
    },
  },
  subCommands: {
    "agent-mode": () => import("./config/agent-mode").then((m) => m.default),
    telemetry: () => import("./config/telemetry").then((m) => m.default),
    get: () => import("./config/get").then((m) => m.default),
    set: () => import("./config/set").then((m) => m.default),
  },
  async run({ args }) {
    if (!process.stdin.isTTY) {
      errorLine("'skilltap config' must be run interactively.");
      process.exit(1);
    }

    const configResult = await loadConfig();
    if (!configResult.ok) {
      errorLine(configResult.error.message, configResult.error.hint);
      process.exit(1);
    }
    const existing = configResult.value;

    if (args.reset) {
      const confirmed = await confirm({
        message: "Overwrite existing config?",
        initialValue: false,
      });
      if (isCancel(confirmed) || !confirmed) {
        cancel("Cancelled.");
        process.exit(2);
      }
    }

    intro("Welcome to skilltap setup!");

    const result = await group(
      {
        scope: () =>
          select({
            message: "Default install scope?",
            options: [
              { value: "", label: "Ask each time" },
              { value: "global", label: "Always global" },
              { value: "project", label: "Always project" },
            ],
            initialValue: existing.defaults.scope,
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
            initialValues: existing.defaults.also,
            required: false,
          }),

        scan: () =>
          select({
            message: "Security scan level?",
            options: [
              {
                value: "static",
                label: "Static only",
                hint: "fast, catches common attacks",
              },
              {
                value: "semantic",
                label: "Static + Semantic",
                hint: "thorough, uses your agent CLI",
              },
              { value: "off", label: "Off", hint: "not recommended" },
            ],
            initialValue: existing.security.scan,
          }),

        agent: ({ results }) => {
          if (results.scan !== "semantic")
            return Promise.resolve(existing.security.agent);
          return selectAgentForConfig(existing.security.agent);
        },

        onWarn: () =>
          select({
            message: "When security warnings are found?",
            options: [
              { value: "prompt", label: "Ask me to decide" },
              { value: "fail", label: "Always block (strict)" },
            ],
            initialValue: existing.security.on_warn,
          }),

        telemetry: () =>
          confirm({
            message:
              "Share anonymous usage data? (OS, arch, command success/fail — no skill names or paths. Never sold.)",
            initialValue: existing.telemetry.enabled,
          }),
      },
      {
        onCancel() {
          cancel("Setup cancelled.");
          process.exit(2);
        },
      },
    );

    const telemetryEnabled = result.telemetry as boolean;
    const anonymousId = telemetryEnabled
      ? existing.telemetry.anonymous_id || crypto.randomUUID()
      : existing.telemetry.anonymous_id;

    const newConfig: Config = {
      ...existing,
      defaults: {
        ...existing.defaults,
        scope: result.scope as "" | "global" | "project",
        also: result.also as string[],
      },
      security: {
        ...existing.security,
        scan: result.scan as "static" | "semantic" | "off",
        on_warn: result.onWarn as "prompt" | "fail",
        agent: result.agent as string,
      },
      telemetry: {
        ...existing.telemetry,
        enabled: telemetryEnabled,
        anonymous_id: anonymousId,
        notice_shown: true,
      },
    };

    const saveResult = await saveConfig(newConfig);
    if (!saveResult.ok) {
      errorLine(saveResult.error.message);
      process.exit(1);
    }

    outro(`Wrote ${getConfigDir()}/config.toml`);
  },
});
