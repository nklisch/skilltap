#!/usr/bin/env bun
import { defineCommand, runMain } from "citty";
import { VERSION } from "@skilltap/core";
import { tryFindProjectRoot } from "./ui/resolve";

// Handle --get-completions before citty takes over — fast path for tab completion
if (process.argv.includes("--get-completions")) {
  const idx = process.argv.indexOf("--get-completions");
  const type = process.argv[idx + 1] ?? "";
  const { printCompletions } = await import("./completions/dynamic");
  await printCompletions(type);
  process.exit(0);
}

// ─── Footer bar ──────────────────────────────────────────────────────────────
// Persistent hint bar at the bottom of the terminal. Invisible when idle,
// auto-updates when any prompt is active. No-op on non-TTY.
import { footer } from "./ui/footer";
footer().open();

// ─── Startup checks ───────────────────────────────────────────────────────────
// Skip for --version, --help, self-update, and telemetry subcommand
const SKIP_STARTUP_ARGS = new Set([
  "--version",
  "--help",
  "-h",
  "self-update",
  "telemetry",
  "status",
]);

// These commands handle telemetry consent themselves — skip the startup prompt for them
const SKIP_TELEMETRY_NOTICE_ARGS = new Set([...SKIP_STARTUP_ARGS, "config"]);
const shouldRunStartup =
  !process.env.SKILLTAP_NO_STARTUP &&
  !process.argv.slice(2).some((a) => SKIP_STARTUP_ARGS.has(a));

if (shouldRunStartup) {
  await runStartupUpdateCheck();
  await runStartupSkillUpdateCheck();
  const shouldRunTelemetryNotice = !process.argv.slice(2).some((a) =>
    SKIP_TELEMETRY_NOTICE_ARGS.has(a),
  );
  if (shouldRunTelemetryNotice) {
    await sendFirstRunPing();
    await runTelemetryNotice();
  }
}

async function sendFirstRunPing(): Promise<void> {
  const { loadConfig } = await import("@skilltap/core");
  const configResult = await loadConfig();
  if (!configResult.ok) return;
  const config = configResult.value;

  // Already shown the notice once — this is not a first run
  if (config.telemetry.notice_shown) return;

  // Minimal anonymous ping: no client_id, no UUID — just OS/arch/version
  const { sendFirstRun } = await import("./telemetry");
  sendFirstRun(VERSION);
}

async function runTelemetryNotice(): Promise<void> {
  const { loadConfig, saveConfig } = await import("@skilltap/core");
  const configResult = await loadConfig();
  if (!configResult.ok) return;
  const config = configResult.value;

  if (config["agent-mode"].enabled) return;
  if (process.env.CI) return;
  if (config.telemetry.notice_shown) return;
  if (process.env.DO_NOT_TRACK === "1" || process.env.SKILLTAP_TELEMETRY_DISABLED === "1") {
    // Mark shown so we don't re-display on every run
    const updated = { ...config, telemetry: { ...config.telemetry, notice_shown: true } };
    await saveConfig(updated);
    return;
  }

  if (process.stdin.isTTY && process.stderr.isTTY) {
    // Interactive: ask the user directly
    const { isCancel } = await import("@clack/prompts");
    const { footerConfirm: confirm } = await import("./ui/footer");
    process.stderr.write("\n");
    const opted = await confirm({
      message:
        "Share anonymous usage data? (OS, arch, command success/fail — no skill names or paths. Never sold.)",
      initialValue: false,
    });
    const enabled = !isCancel(opted) && opted === true;
    const anonymousId = enabled
      ? config.telemetry.anonymous_id || crypto.randomUUID()
      : config.telemetry.anonymous_id;
    const updated = {
      ...config,
      telemetry: { ...config.telemetry, enabled, anonymous_id: anonymousId, notice_shown: true },
    };
    await saveConfig(updated);

    if (enabled) {
      const { sendEvent, telemetryBase } = await import("./telemetry");
      sendEvent(updated, "skilltap_installed", {
        ...telemetryBase(false),
        version: VERSION,
      });
    }

    process.stderr.write("\n");
  } else {
    // Non-interactive: show banner, don't enable
    process.stderr.write(
      "\n┌─ Telemetry Notice ─────────────────────────────────────────────────────┐\n" +
      "│ skilltap can send anonymous usage data (OS, arch, command              │\n" +
      "│ success/fail). No skill names, paths, or personal info collected.      │\n" +
      "│ Data is never sold.                                                    │\n" +
      "│                                                                        │\n" +
      "│ Run 'skilltap telemetry enable' to opt in.                             │\n" +
      "│ Set DO_NOT_TRACK=1 to silence this notice without opting in.           │\n" +
      "└────────────────────────────────────────────────────────────────────────┘\n\n",
    );
    const updated = { ...config, telemetry: { ...config.telemetry, notice_shown: true } };
    await saveConfig(updated);
  }
}

async function runStartupUpdateCheck(): Promise<void> {
  const { checkForUpdate, downloadAndInstall, isCompiledBinary, loadConfig } =
    await import("@skilltap/core");

  // Load config for update preferences — fall back gracefully if it fails
  const configResult = await loadConfig();
  const config = configResult.ok ? configResult.value : null;

  // Suppress update output when running in agent mode
  if (config?.["agent-mode"]?.enabled) return;

  const intervalHours = config?.updates?.interval_hours ?? 24;
  const autoUpdate = config?.updates?.auto_update ?? "off";

  const result = await checkForUpdate(VERSION, intervalHours);
  if (!result) return;

  const { current, latest, type } = result;

  const autoUpdateCoversType =
    (autoUpdate === "patch" && type === "patch") ||
    (autoUpdate === "minor" && (type === "patch" || type === "minor"));

  // Major releases are never auto-updated — always just notify
  if (autoUpdateCoversType && isCompiledBinary()) {
    process.stderr.write(`⟳  Auto-updating skilltap ${current} → ${latest} (${type})…\n`);
    const installResult = await downloadAndInstall(latest);
    if (installResult.ok) {
      process.stderr.write(`✓  Updated to v${latest}. Changes take effect next run.\n\n`);
    } else {
      // Update failed — fall through to notify instead
      printUpdateNotice(current, latest, type);
    }
    return;
  }

  printUpdateNotice(current, latest, type);
}

async function runStartupSkillUpdateCheck(): Promise<void> {
  const { checkForSkillUpdates, loadConfig } =
    await import("@skilltap/core");

  const configResult = await loadConfig();
  const config = configResult.ok ? configResult.value : null;

  if (config?.["agent-mode"]?.enabled) return;

  const intervalHours = config?.updates?.skill_check_interval_hours ?? 24;
  const projectRoot = await tryFindProjectRoot();

  const updates = await checkForSkillUpdates(intervalHours, projectRoot);
  if (!updates || updates.length === 0) return;

  printSkillUpdateNotice(updates);
}

function printSkillUpdateNotice(names: string[]): void {
  const DIM = "\x1b[2m";
  const RESET = "\x1b[0m";

  const nameList =
    names.length <= 3
      ? ` (${names.join(", ")})`
      : "";
  const count = names.length === 1 ? "1 skill update" : `${names.length} skill updates`;
  process.stderr.write(
    `${DIM}↑  ${count} available${nameList}. Run: skilltap update${RESET}\n\n`,
  );
}

function printUpdateNotice(
  current: string,
  latest: string,
  type: "patch" | "minor" | "major",
): void {
  const DIM = "\x1b[2m";
  const YELLOW = "\x1b[33m";
  const BOLD = "\x1b[1m";
  const RESET = "\x1b[0m";

  if (type === "major") {
    process.stderr.write(
      `${YELLOW}${BOLD}⚠  Major update available: v${current} → v${latest}${RESET}  ` +
        `${DIM}Breaking changes may apply. Run: skilltap self-update${RESET}\n\n`,
    );
  } else if (type === "minor") {
    process.stderr.write(
      `${BOLD}↑  Update available: v${current} → v${latest}${RESET}  ` +
        `${DIM}(${type}) Run: skilltap self-update${RESET}\n\n`,
    );
  } else {
    // patch — subtle
    process.stderr.write(
      `${DIM}↑  skilltap ${current} → ${latest} available. Run: skilltap self-update${RESET}\n\n`,
    );
  }
}

// ─── CLI definition ───────────────────────────────────────────────────────────

const main = defineCommand({
  meta: {
    name: "skilltap",
    version: VERSION,
    description: "Install agent skills from any git host",
  },
  subCommands: {
    status: () => import("./commands/status").then((m) => m.default),
    install: () => import("./commands/install").then((m) => m.default),
    update: () => import("./commands/update").then((m) => m.default),
    find: () => import("./commands/find").then((m) => m.default),
    skills: () => import("./commands/skills/index").then((m) => m.default),

    // Silent aliases — route to new locations under skills/
    list: () => import("./commands/skills/index").then((m) => m.default),
    remove: () => import("./commands/skills/remove").then((m) => m.default),
    info: () => import("./commands/skills/info").then((m) => m.default),
    link: () => import("./commands/skills/link").then((m) => m.default),
    unlink: () => import("./commands/skills/unlink").then((m) => m.default),

    create: () => import("./commands/create").then((m) => m.default),
    verify: () => import("./commands/verify").then((m) => m.default),
    doctor: () => import("./commands/doctor").then((m) => m.default),
    config: () => import("./commands/config").then((m) => m.default),
    "self-update": () =>
      import("./commands/self-update").then((m) => m.default),
    completions: () =>
      import("./commands/completions").then((m) => m.default),
    tap: defineCommand({
      meta: {
        name: "tap",
        description: "Manage taps",
      },
      subCommands: {
        add: () => import("./commands/tap/add").then((m) => m.default),
        remove: () => import("./commands/tap/remove").then((m) => m.default),
        list: () => import("./commands/tap/list").then((m) => m.default),
        info: () => import("./commands/tap/info").then((m) => m.default),
        init: () => import("./commands/tap/init").then((m) => m.default),
        install: () =>
          import("./commands/tap/install").then((m) => m.default),
      },
    }),
  },
});

runMain(main);
