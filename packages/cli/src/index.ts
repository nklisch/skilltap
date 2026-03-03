#!/usr/bin/env bun
import { defineCommand, runMain } from "citty";
import { VERSION } from "@skilltap/core";

// Handle --get-completions before citty takes over — fast path for tab completion
if (process.argv.includes("--get-completions")) {
  const idx = process.argv.indexOf("--get-completions");
  const type = process.argv[idx + 1] ?? "";
  const { printCompletions } = await import("./completions/dynamic");
  await printCompletions(type);
  process.exit(0);
}

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
const shouldRunStartup = !process.argv.slice(2).some((a) =>
  SKIP_STARTUP_ARGS.has(a),
);

if (shouldRunStartup) {
  await runStartupUpdateCheck();
  await runTelemetryNotice();
}

async function runTelemetryNotice(): Promise<void> {
  const { loadConfig, saveConfig } = await import("@skilltap/core");
  const configResult = await loadConfig();
  if (!configResult.ok) return;
  const config = configResult.value;

  if (config["agent-mode"].enabled) return;
  if (process.env.CI) return;
  if (config.telemetry.notice_shown) return;

  // Show once, then mark as shown
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

  const updated = {
    ...config,
    telemetry: { ...config.telemetry, notice_shown: true },
  };
  await saveConfig(updated);
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
  if (autoUpdateCoversType && type !== "major" && isCompiledBinary()) {
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
    remove: () => import("./commands/remove").then((m) => m.default),
    list: () => import("./commands/list").then((m) => m.default),
    update: () => import("./commands/update").then((m) => m.default),
    find: () => import("./commands/find").then((m) => m.default),
    link: () => import("./commands/link").then((m) => m.default),
    unlink: () => import("./commands/unlink").then((m) => m.default),
    info: () => import("./commands/info").then((m) => m.default),
    create: () => import("./commands/create").then((m) => m.default),
    verify: () => import("./commands/verify").then((m) => m.default),
    doctor: () => import("./commands/doctor").then((m) => m.default),
    config: () => import("./commands/config").then((m) => m.default),
    telemetry: () => import("./commands/telemetry").then((m) => m.default),
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
        update: () => import("./commands/tap/update").then((m) => m.default),
        init: () => import("./commands/tap/init").then((m) => m.default),
      },
    }),
  },
});

runMain(main);
