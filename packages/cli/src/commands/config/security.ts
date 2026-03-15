import { cancel, intro, isCancel, note, outro } from "@clack/prompts";
import {
  footerConfirm as confirm,
  footerSelect as select,
  footerText as text,
} from "../../ui/footer";
import {
  PRESET_VALUES,
  SCAN_MODES,
  ON_WARN_MODES,
  SECURITY_PRESETS,
  SOURCE_TYPES,
  loadConfig,
  saveConfig,
  getConfigDir,
  describeSecurityMode,
  type Config,
  type SecurityMode,
  type TrustOverride,
} from "@skilltap/core";
import { defineCommand } from "citty";
import { errorLine } from "../../ui/format";
import { selectAgentForConfig, SCAN_MODE_OPTIONS } from "../../ui/prompts";

// ─── Non-interactive helpers ───────────────────────────────────────────────

function isNonInteractive(args: {
  preset?: string;
  mode?: string;
  scan?: string;
  "on-warn"?: string;
  "require-scan"?: boolean;
  trust?: string;
  "remove-trust"?: string;
}): boolean {
  return (
    args.preset !== undefined ||
    args.scan !== undefined ||
    args["on-warn"] !== undefined ||
    args["require-scan"] !== undefined ||
    args.trust !== undefined ||
    args["remove-trust"] !== undefined
  );
}

function applyPresetToMode(config: Config, preset: (typeof SECURITY_PRESETS)[number], target: "human" | "agent" | "both"): void {
  const values = PRESET_VALUES[preset];
  if (target === "human" || target === "both") {
    config.security.human = { ...config.security.human, ...values };
  }
  if (target === "agent" || target === "both") {
    config.security.agent = { ...config.security.agent, ...values };
  }
}

function parseTarget(modeArg: string | undefined): "human" | "agent" | "both" | null {
  if (!modeArg || modeArg === "both") return "both";
  if (modeArg === "human" || modeArg === "agent") return modeArg;
  return null;
}

function parseTrustFlag(trust: string): TrustOverride | null {
  // Format: tap:<name>=<preset> or source:<type>=<preset>
  const tapMatch = trust.match(/^tap:([^=]+)=([^=]+)$/);
  if (tapMatch) {
    const preset = tapMatch[2] as string;
    if (!(SECURITY_PRESETS as readonly string[]).includes(preset)) return null;
    return { match: tapMatch[1] as string, kind: "tap", preset: preset as (typeof SECURITY_PRESETS)[number] };
  }
  const sourceMatch = trust.match(/^source:([^=]+)=([^=]+)$/);
  if (sourceMatch) {
    const sourceType = sourceMatch[1] as string;
    const preset = sourceMatch[2] as string;
    if (!(SOURCE_TYPES as readonly string[]).includes(sourceType)) return null;
    if (!(SECURITY_PRESETS as readonly string[]).includes(preset)) return null;
    return { match: sourceType, kind: "source", preset: preset as (typeof SECURITY_PRESETS)[number] };
  }
  return null;
}

async function runNonInteractive(
  args: {
    preset?: string;
    mode?: string;
    scan?: string;
    "on-warn"?: string;
    "require-scan"?: boolean;
    trust?: string;
    "remove-trust"?: string;
  },
): Promise<void> {
  const configResult = await loadConfig();
  if (!configResult.ok) {
    errorLine(configResult.error.message, configResult.error.hint);
    process.exit(1);
  }
  const config = configResult.value;

  const target = parseTarget(args.mode);
  if (target === null) {
    errorLine(`Invalid mode: '${args.mode}'. Use: human, agent, or both`);
    process.exit(1);
  }

  if (args["remove-trust"]) {
    const name = args["remove-trust"];
    const idx = config.security.overrides.findIndex((o) => o.match === name);
    if (idx === -1) {
      errorLine(`No trust override found with match '${name}'`);
      process.exit(1);
    }
    config.security.overrides.splice(idx, 1);
    const saveResult = await saveConfig(config);
    if (!saveResult.ok) {
      errorLine(saveResult.error.message);
      process.exit(1);
    }
    process.stdout.write(`OK: removed trust override '${name}'\n`);
    return;
  }

  if (args.trust) {
    const override = parseTrustFlag(args.trust);
    if (!override) {
      errorLine(
        `Invalid --trust format: '${args.trust}'\n  Expected: tap:<name>=<preset> or source:<type>=<preset>\n  Presets: ${SECURITY_PRESETS.join(", ")}\n  Source types: ${SOURCE_TYPES.join(", ")}`,
      );
      process.exit(1);
    }
    config.security.overrides.push(override);
    const saveResult = await saveConfig(config);
    if (!saveResult.ok) {
      errorLine(saveResult.error.message);
      process.exit(1);
    }
    process.stdout.write(
      `OK: added ${override.kind} trust override '${override.match}' → ${override.preset}\n`,
    );
    return;
  }

  if (args.preset) {
    if (!(SECURITY_PRESETS as readonly string[]).includes(args.preset)) {
      errorLine(
        `Invalid preset: '${args.preset}'. Valid presets: ${SECURITY_PRESETS.join(", ")}`,
      );
      process.exit(1);
    }
    const preset = args.preset as (typeof SECURITY_PRESETS)[number];
    applyPresetToMode(config, preset, target);
  }

  if (args.scan) {
    if (!(SCAN_MODES as readonly string[]).includes(args.scan)) {
      errorLine(`Invalid scan level: '${args.scan}'. Valid: ${SCAN_MODES.join(", ")}`);
      process.exit(1);
    }
    const scan = args.scan as (typeof SCAN_MODES)[number];
    if (target === "human" || target === "both") config.security.human.scan = scan;
    if (target === "agent" || target === "both") config.security.agent.scan = scan;
  }

  if (args["on-warn"]) {
    if (!(ON_WARN_MODES as readonly string[]).includes(args["on-warn"])) {
      errorLine(`Invalid on-warn value: '${args["on-warn"]}'. Valid: ${ON_WARN_MODES.join(", ")}`);
      process.exit(1);
    }
    const onWarn = args["on-warn"] as (typeof ON_WARN_MODES)[number];
    if (target === "human" || target === "both") config.security.human.on_warn = onWarn;
    if (target === "agent" || target === "both") config.security.agent.on_warn = onWarn;
  }

  if (args["require-scan"] !== undefined) {
    if (target === "human" || target === "both") config.security.human.require_scan = args["require-scan"];
    if (target === "agent" || target === "both") config.security.agent.require_scan = args["require-scan"];
  }

  const saveResult = await saveConfig(config);
  if (!saveResult.ok) {
    errorLine(saveResult.error.message);
    process.exit(1);
  }

  if (target === "human" || target === "both") {
    process.stdout.write(`OK: security.human = ${describeSecurityMode(config.security.human)}\n`);
  }
  if (target === "agent" || target === "both") {
    process.stdout.write(`OK: security.agent = ${describeSecurityMode(config.security.agent)}\n`);
  }
}

// ─── Preset select options ─────────────────────────────────────────────────

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

// ─── Interactive wizard helpers ────────────────────────────────────────────

async function promptSecurityMode(
  label: string,
  current: SecurityMode,
  agentCli: string,
): Promise<{ mode: SecurityMode; agentCli: string }> {
  const presetResult = await select({
    message: `Security preset for ${label}?`,
    options: PRESET_OPTIONS,
    initialValue: "standard",
  });
  if (isCancel(presetResult)) {
    cancel("Cancelled.");
    process.exit(2);
  }

  const chosenPreset = presetResult as string;

  if (chosenPreset !== "custom") {
    const preset = chosenPreset as (typeof SECURITY_PRESETS)[number];
    const values = PRESET_VALUES[preset];
    let newAgentCli = agentCli;

    if (values.scan === "semantic") {
      newAgentCli = await selectAgentForConfig(agentCli);
    }

    return { mode: { ...values }, agentCli: newAgentCli };
  }

  // Custom path
  const scanResult = await select({
    message: "Scan level?",
    options: SCAN_MODE_OPTIONS,
    initialValue: current.scan,
  });
  if (isCancel(scanResult)) {
    cancel("Cancelled.");
    process.exit(2);
  }

  const onWarnResult = await select({
    message: "When warnings are found?",
    options: ON_WARN_OPTIONS,
    initialValue: current.on_warn,
  });
  if (isCancel(onWarnResult)) {
    cancel("Cancelled.");
    process.exit(2);
  }

  const requireScanResult = await confirm({
    message: "Require scanning? (block --skip-scan)",
    initialValue: current.require_scan,
  });
  if (isCancel(requireScanResult)) {
    cancel("Cancelled.");
    process.exit(2);
  }

  let newAgentCli = agentCli;
  if (scanResult === "semantic") {
    newAgentCli = await selectAgentForConfig(agentCli);
  }

  return {
    mode: {
      scan: scanResult as (typeof SCAN_MODES)[number],
      on_warn: onWarnResult as (typeof ON_WARN_MODES)[number],
      require_scan: requireScanResult as boolean,
    },
    agentCli: newAgentCli,
  };
}

// ─── Interactive override loop ─────────────────────────────────────────────

const SOURCE_TYPE_OVERRIDE_OPTIONS = [
  { value: "tap", label: "A specific tap" },
  { value: "git", label: "All git URL sources" },
  { value: "npm", label: "All npm sources" },
  { value: "local", label: "All local path sources" },
  { value: "done", label: "Done adding overrides" },
];

const PRESET_ONLY_OPTIONS = PRESET_OPTIONS.filter((o) => o.value !== "custom");

async function promptTrustOverrides(current: TrustOverride[]): Promise<TrustOverride[]> {
  const configureOverrides = await confirm({
    message: "Configure trust overrides?",
    initialValue: false,
  });
  if (isCancel(configureOverrides)) {
    cancel("Cancelled.");
    process.exit(2);
  }
  if (!configureOverrides) return current;

  const overrides = [...current];

  while (true) {
    const overrideFor = await select({
      message: "Add override for:",
      options: SOURCE_TYPE_OVERRIDE_OPTIONS,
    });
    if (isCancel(overrideFor)) {
      cancel("Cancelled.");
      process.exit(2);
    }

    if (overrideFor === "done") break;

    let match: string;
    let kind: "tap" | "source";

    if (overrideFor === "tap") {
      const tapName = await text({
        message: "Tap name:",
        validate(v) {
          if (!v) return "Required";
        },
      });
      if (isCancel(tapName)) {
        cancel("Cancelled.");
        process.exit(2);
      }
      match = tapName as string;
      kind = "tap";
    } else {
      match = overrideFor as string;
      kind = "source";
    }

    const presetResult = await select({
      message: `Security preset for "${match}"?`,
      options: PRESET_ONLY_OPTIONS,
    });
    if (isCancel(presetResult)) {
      cancel("Cancelled.");
      process.exit(2);
    }

    overrides.push({
      match,
      kind,
      preset: presetResult as (typeof SECURITY_PRESETS)[number],
    });
  }

  return overrides;
}

// ─── Interactive wizard ────────────────────────────────────────────────────

async function runInteractive(): Promise<void> {
  const configResult = await loadConfig();
  if (!configResult.ok) {
    errorLine(configResult.error.message, configResult.error.hint);
    process.exit(1);
  }
  const config = configResult.value;

  intro("Security Configuration");

  // Step 1: Which mode?
  const modeResult = await select({
    message: "Configure which mode?",
    options: [
      { value: "human", label: "Human", hint: "when you run skilltap" },
      { value: "agent", label: "Agent", hint: "when AI agents run skilltap" },
      { value: "both", label: "Both", hint: "same settings for both" },
    ],
    initialValue: "both",
  });
  if (isCancel(modeResult)) {
    cancel("Cancelled.");
    process.exit(2);
  }
  const target = modeResult as "human" | "agent" | "both";

  let newHuman = { ...config.security.human };
  let newAgent = { ...config.security.agent };
  let newAgentCli = config.security.agent_cli;

  if (target === "both") {
    const { mode, agentCli } = await promptSecurityMode("both modes", config.security.human, newAgentCli);
    newHuman = mode;
    newAgent = { ...mode };
    newAgentCli = agentCli;
  } else if (target === "human") {
    const { mode, agentCli } = await promptSecurityMode("human mode", config.security.human, newAgentCli);
    newHuman = mode;
    newAgentCli = agentCli;
  } else {
    const { mode, agentCli } = await promptSecurityMode("agent mode", config.security.agent, newAgentCli);
    newAgent = mode;
    newAgentCli = agentCli;
  }

  // Step 2: Trust overrides
  const newOverrides = await promptTrustOverrides(config.security.overrides);

  // Step 3: Summary
  let summaryLines =
    `Human: ${describeSecurityMode(newHuman)}\n` +
    `Agent: ${describeSecurityMode(newAgent)}`;

  if (newOverrides.length > 0) {
    summaryLines += "\n\nTrust overrides:";
    for (const o of newOverrides) {
      summaryLines += `\n  ${o.kind === "tap" ? o.match : `${o.match} sources`} → ${o.preset}`;
    }
  }

  note(summaryLines, "Security Summary");

  // Step 4: Confirm save
  const saveConfirm = await confirm({
    message: "Save these settings?",
    initialValue: true,
  });
  if (isCancel(saveConfirm) || !saveConfirm) {
    cancel("Cancelled.");
    process.exit(2);
  }

  config.security = {
    ...config.security,
    human: newHuman,
    agent: newAgent,
    agent_cli: newAgentCli,
    overrides: newOverrides,
  };

  const saveResult = await saveConfig(config);
  if (!saveResult.ok) {
    errorLine(saveResult.error.message);
    process.exit(1);
  }

  outro(`Wrote ${getConfigDir()}/config.toml`);
}

// ─── Command definition ────────────────────────────────────────────────────

export default defineCommand({
  meta: {
    name: "skilltap config security",
    description: "Configure security settings",
  },
  args: {
    preset: { type: "string", description: "Apply a named preset: none, relaxed, standard, strict" },
    mode: { type: "string", description: "Which mode to configure: human, agent, both (default: both)" },
    scan: { type: "string", description: "Scan level: static, semantic, off" },
    "on-warn": { type: "string", description: "Warning behavior: prompt, fail, allow" },
    "require-scan": { type: "boolean", description: "Block --skip-scan" },
    trust: { type: "string", description: "Add trust override: tap:name=preset or source:type=preset" },
    "remove-trust": { type: "string", description: "Remove a trust override by match name" },
  },
  async run({ args }) {
    if (isNonInteractive(args)) {
      await runNonInteractive(args);
      return;
    }

    if (!process.stdin.isTTY) {
      errorLine("'skilltap config security' requires a TTY for interactive mode. Use flags for non-interactive use.");
      process.exit(1);
    }

    await runInteractive();
  },
});
