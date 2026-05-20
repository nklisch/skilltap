#!/usr/bin/env bun
import { basename } from "node:path";
import { VERSION } from "@skilltap/core";
import { defineCommand, runMain } from "citty";
import { tryFindProjectRoot } from "./ui/resolve";

// Handle --get-completions before citty takes over — fast path for tab completion
if (process.argv.includes("--get-completions")) {
  const idx = process.argv.indexOf("--get-completions");
  const type = process.argv[idx + 1] ?? "";
  const { printCompletions } = await import("./completions/dynamic");
  await printCompletions(type);
  process.exit(0);
}

// ─── Internal cache-refresh subcommands ───────────────────────────────────────
// Spawned detached by the startup checks below so the foreground CLI can exit
// immediately. Bun.$ subprocesses keep the parent event loop alive — moving
// the network work into a detached child is what makes commands like `doctor`
// (with warnings) actually exit instead of stalling on background `git fetch`.
if (process.argv[2] === "_refresh-update-cache") {
  const { refreshUpdateCache } = await import("@skilltap/core");
  await refreshUpdateCache();
  process.exit(0);
}
if (process.argv[2] === "_refresh-skill-update-cache") {
  const { refreshSkillUpdateCache } = await import("@skilltap/core");
  const projectRoot =
    process.argv[3] && process.argv[3].length > 0 ? process.argv[3] : null;
  await refreshSkillUpdateCache(projectRoot);
  process.exit(0);
}

/**
 * Spawn the running skilltap binary (or `bun <script>` in dev mode) detached.
 * The child inherits no stdio and is unref'd, so the parent process can exit
 * immediately without waiting for the child's network calls.
 */
function spawnSelfDetached(args: string[]): void {
  const isCompiled = !["bun", "bun.exe"].includes(basename(process.execPath));
  const cmd = isCompiled
    ? [process.execPath, ...args]
    : process.argv[1]
      ? [process.execPath, process.argv[1], ...args]
      : null;
  if (!cmd) return;
  try {
    const proc = Bun.spawn(cmd, {
      stdio: ["ignore", "ignore", "ignore"],
      env: { ...process.env, SKILLTAP_NO_STARTUP: "1" },
    });
    proc.unref();
  } catch {
    // Best-effort — never block the CLI on a failed background spawn
  }
}

import { createOutput } from "./output";
// ─── Footer bar ──────────────────────────────────────────────────────────────
// Persistent hint bar at the bottom of the terminal. Invisible when idle,
// auto-updates when any prompt is active. No-op on non-TTY.
import { footer } from "./ui/footer";

if (!process.env.SKILLTAP_NO_STARTUP) {
  footer().open();
}

// ─── Ctrl+C always exits ─────────────────────────────────────────────────────
// Clack prompts put stdin in raw mode, catching Ctrl+C as a cancel symbol.
// Outside prompts, SIGINT fires normally — ensure we always exit cleanly.
process.on("SIGINT", () => {
  footer().close();
  process.exit(130);
});

// ─── Startup checks ───────────────────────────────────────────────────────────
// Skip for --version, --help, self-update, and telemetry subcommand
const SKIP_STARTUP_ARGS = new Set([
  "--version",
  "--help",
  "-h",
  "self-update",
  "telemetry",
  "status",
  "migrate",
]);

// These commands handle telemetry consent themselves — skip the startup prompt for them
const SKIP_TELEMETRY_NOTICE_ARGS = new Set([...SKIP_STARTUP_ARGS, "config"]);
const shouldRunStartup =
  !process.env.SKILLTAP_NO_STARTUP &&
  !process.argv.slice(2).some((a) => SKIP_STARTUP_ARGS.has(a));

if (shouldRunStartup) {
  await runLegacyStateDetectionNotice();
  await runStartupUpdateCheck();
  await runStartupSkillUpdateCheck();
  const shouldRunTelemetryNotice = !process.argv
    .slice(2)
    .some((a) => SKIP_TELEMETRY_NOTICE_ARGS.has(a));
  if (shouldRunTelemetryNotice) {
    await sendFirstRunPing();
    await runTelemetryNotice();
  }
}

// Legacy-state soft hint: if pre-state.json markers exist and no state.json
// is present yet, suggest `skilltap migrate`.
async function runLegacyStateDetectionNotice(): Promise<void> {
  try {
    const { detectV1StateGlobal, hasAnyV1Markers, getStatePath } = await import(
      "@skilltap/core"
    );
    const markers = await detectV1StateGlobal();
    if (!hasAnyV1Markers(markers)) return;
    const stateFile = Bun.file(getStatePath());
    if (await stateFile.exists()) return;
    const out = createOutput({ json: false, quiet: false });
    out.block(
      ["↑  Legacy state detected. Run 'skilltap migrate' to upgrade.", ""],
      { stream: "stderr" },
    );
  } catch {
    // Detection is best-effort. Never block startup.
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

  if (process.env.CI) return;
  if (config.telemetry.notice_shown) return;
  if (
    process.env.DO_NOT_TRACK === "1" ||
    process.env.SKILLTAP_TELEMETRY_DISABLED === "1"
  ) {
    // Mark shown so we don't re-display on every run
    const updated = {
      ...config,
      telemetry: { ...config.telemetry, notice_shown: true },
    };
    await saveConfig(updated);
    return;
  }

  const noticeOut = createOutput({ json: false, quiet: false });

  if (process.stdin.isTTY && process.stderr.isTTY) {
    // Interactive: ask the user directly
    const { isCancel } = await import("@clack/prompts");
    const { footerConfirm: confirm } = await import("./ui/footer");
    noticeOut.block([""], { stream: "stderr" }); // blank line before interactive prompt
    const opted = await confirm({
      message:
        "Share anonymous usage data? (OS, arch, command success/fail — no skill names or paths. Never sold.)",
      initialValue: false,
    });
    if (isCancel(opted)) process.exit(130);
    const enabled = opted === true;
    const anonymousId = enabled
      ? config.telemetry.anonymous_id || crypto.randomUUID()
      : config.telemetry.anonymous_id;
    const updated = {
      ...config,
      telemetry: {
        ...config.telemetry,
        enabled,
        anonymous_id: anonymousId,
        notice_shown: true,
      },
    };
    await saveConfig(updated);

    if (enabled) {
      const { sendEvent, telemetryBase } = await import("./telemetry");
      sendEvent(updated, "skilltap_installed", {
        ...telemetryBase(),
        version: VERSION,
      });
    }

    noticeOut.block([""], { stream: "stderr" }); // blank line after interactive prompt
  } else {
    // Non-interactive: show banner, don't enable
    const out = createOutput({ json: false, quiet: false });
    out.block(
      [
        "",
        "┌─ Telemetry Notice ─────────────────────────────────────────────────────┐",
        "│ skilltap can send anonymous usage data (OS, arch, command              │",
        "│ success/fail). No skill names, paths, or personal info collected.      │",
        "│ Data is never sold.                                                    │",
        "│                                                                        │",
        "│ Run 'skilltap telemetry enable' to opt in.                             │",
        "│ Set DO_NOT_TRACK=1 to silence this notice without opting in.           │",
        "└────────────────────────────────────────────────────────────────────────┘",
        "",
      ],
      { stream: "stderr" },
    );
    const updated = {
      ...config,
      telemetry: { ...config.telemetry, notice_shown: true },
    };
    await saveConfig(updated);
  }
}

async function runStartupUpdateCheck(): Promise<void> {
  const {
    checkForUpdate,
    downloadAndInstall,
    isCompiledBinary,
    isUpdateCacheStale,
    loadConfig,
  } = await import("@skilltap/core");

  // Load config for update preferences — fall back gracefully if it fails
  const configResult = await loadConfig();
  const config = configResult.ok ? configResult.value : null;

  const intervalHours = config?.updates?.interval_hours ?? 24;
  const autoUpdate = config?.updates?.auto_update ?? "off";

  // Kick off a detached cache refresh when stale — never blocks the CLI.
  if (await isUpdateCacheStale(intervalHours)) {
    spawnSelfDetached(["_refresh-update-cache"]);
  }

  const result = await checkForUpdate(VERSION, intervalHours);
  if (!result) return;

  const { current, latest, type } = result;

  const autoUpdateCoversType =
    (autoUpdate === "patch" && type === "patch") ||
    (autoUpdate === "minor" && (type === "patch" || type === "minor"));

  // Major releases are never auto-updated — always just notify
  if (autoUpdateCoversType && isCompiledBinary()) {
    const out = createOutput({ json: false, quiet: false });
    out.block([`⟳  Auto-updating skilltap ${current} → ${latest} (${type})…`], {
      stream: "stderr",
    });
    const installResult = await downloadAndInstall(latest);
    if (installResult.ok) {
      out.block(
        [`✓  Updated to v${latest}. Changes take effect next run.`, ""],
        { stream: "stderr" },
      );
    } else {
      // Update failed — fall through to notify instead
      printUpdateNotice(current, latest, type);
    }
    return;
  }

  printUpdateNotice(current, latest, type);
}

async function runStartupSkillUpdateCheck(): Promise<void> {
  const { checkForSkillUpdates, isSkillUpdateCacheStale, loadConfig } =
    await import("@skilltap/core");

  const configResult = await loadConfig();
  const config = configResult.ok ? configResult.value : null;

  const intervalHours = config?.updates?.skill_check_interval_hours ?? 24;
  const projectRoot = await tryFindProjectRoot();

  // Kick off a detached cache refresh when stale — never blocks the CLI.
  if (await isSkillUpdateCacheStale(intervalHours, projectRoot)) {
    spawnSelfDetached(["_refresh-skill-update-cache", projectRoot ?? ""]);
  }

  const updates = await checkForSkillUpdates(intervalHours, projectRoot);
  if (!updates || updates.length === 0) return;

  printSkillUpdateNotice(updates);
}

function printSkillUpdateNotice(names: string[]): void {
  const DIM = "\x1b[2m";
  const RESET = "\x1b[0m";

  const nameList = names.length <= 3 ? ` (${names.join(", ")})` : "";
  const count =
    names.length === 1 ? "1 skill update" : `${names.length} skill updates`;
  const out = createOutput({ json: false, quiet: false });
  out.block(
    [
      `${DIM}↑  ${count} available${nameList}. Run: skilltap update${RESET}`,
      "",
    ],
    { stream: "stderr" },
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

  const out = createOutput({ json: false, quiet: false });
  if (type === "major") {
    out.block(
      [
        `${YELLOW}${BOLD}⚠  Major update available: v${current} → v${latest}${RESET}  ${DIM}Breaking changes may apply. Run: skilltap self-update${RESET}`,
        "",
      ],
      { stream: "stderr" },
    );
  } else if (type === "minor") {
    out.block(
      [
        `${BOLD}↑  Update available: v${current} → v${latest}${RESET}  ${DIM}(${type}) Run: skilltap self-update${RESET}`,
        "",
      ],
      { stream: "stderr" },
    );
  } else {
    // patch — subtle
    out.block(
      [
        `${DIM}↑  skilltap ${current} → ${latest} available. Run: skilltap self-update${RESET}`,
        "",
      ],
      { stream: "stderr" },
    );
  }
}

// ─── Removed-command hints ───────────────────────────────────────────────────
// Top-level commands removed in the v2.0/v2.2 cleanup. Citty's default
// "unknown command" path falls through to the help banner, which buries the
// signal that the user typed a verb that no longer exists. Intercept these
// names before citty runs so each prints a precise replacement hint.
const REMOVED_COMMANDS: Record<string, { hint: string }> = {
  verify: {
    hint: "Use `skilltap doctor skill <path>` to validate a skill, or `skilltap doctor plugin <path>` for a plugin.",
  },
  link: {
    hint: "Use `skilltap adopt <path>` to track an existing local skill or plugin in place.",
  },
  unlink: {
    hint: "Use `skilltap remove <type> <name>` to detach an installed skill, plugin, or mcp.",
  },
  enable: {
    hint: "Use `skilltap toggle <type> <name>` (or `toggle <type> <name>:<component>`) to re-enable a disabled item.",
  },
  disable: {
    hint: "Use `skilltap toggle <type> <name>` to disable an installed item.",
  },
  skills: {
    hint: "Use `skilltap status` (and the typed `install`/`remove`/`update`/`toggle` subcommands).",
  },
};

const removedCmd = process.argv[2];
if (removedCmd && Object.hasOwn(REMOVED_COMMANDS, removedCmd)) {
  const entry = REMOVED_COMMANDS[removedCmd];
  if (entry) {
    process.stderr.write(
      `Error: \`skilltap ${removedCmd}\` was removed.\n  hint: ${entry.hint}\n`,
    );
    process.exit(1);
  }
}

// Bare `skilltap` invocation: open TUI in TTY, error with hint when piped.
// Detected up front so citty doesn't double-run both main's `run` and a subcommand.
if (process.argv.length === 2) {
  process.stderr.write("[DEBUG INDEX]: Entering bare invocation branch\n");
  if (process.stdout.isTTY) {
    process.stderr.write("[DEBUG INDEX]: stdout is TTY, importing tui\n");
    const { mountTui } = await import("./tui");
    process.stderr.write("[DEBUG INDEX]: tui imported, calling mountTui\n");
    await mountTui("dashboard");
    process.stderr.write("[DEBUG INDEX]: mountTui finished\n");
    process.exit(0);
  } else {
    process.stderr.write("[DEBUG INDEX]: stdout is NOT TTY\n");
    process.stderr.write(
      "skilltap requires a TTY for the dashboard.\n" +
        "  hint: Run `skilltap status` for headless output.\n",
    );
    process.exit(1);
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
    install: () =>
      import("./commands/install/index").then((m) => m.installCommand),
    remove: () =>
      import("./commands/remove/index").then((m) => m.removeCommand),
    update: () => import("./commands/update").then((m) => m.default),
    find: () => import("./commands/find").then((m) => m.default),
    create: () => import("./commands/create").then((m) => m.default),
    doctor: () => import("./commands/doctor").then((m) => m.default),
    migrate: () => import("./commands/migrate").then((m) => m.default),
    sync: () => import("./commands/sync").then((m) => m.default),
    try: () => import("./commands/try").then((m) => m.default),
    toggle: () => import("./commands/toggle").then((m) => m.default),
    adopt: () => import("./commands/adopt").then((m) => m.default),
    move: () => import("./commands/move").then((m) => m.default),
    info: () => import("./commands/info").then((m) => m.default),
    config: () => import("./commands/config").then((m) => m.default),
    "self-update": () =>
      import("./commands/self-update").then((m) => m.default),
    completions: () => import("./commands/completions").then((m) => m.default),
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
      },
    }),
  },
});

runMain(main);
