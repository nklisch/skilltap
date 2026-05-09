import { cancel, intro, isCancel, note, outro } from "@clack/prompts";
import {
  describeSecurityMode,
  getConfigDir,
  loadConfig,
  ON_WARN_MODES,
  type Output,
  saveConfig,
  SCAN_MODES,
} from "@skilltap/core";
import { defineCommand } from "citty";
import {
  footerConfirm as confirm,
  footerSelect as select,
  footerText as text,
} from "../../ui/footer";
import { setupOutput } from "../../ui/setup";

type Args = {
  scan?: string;
  "on-warn"?: string;
  "trust-add"?: string;
  "trust-remove"?: string;
  "trust-list"?: boolean;
};

function isNonInteractive(args: Args): boolean {
  return (
    args.scan !== undefined ||
    args["on-warn"] !== undefined ||
    args["trust-add"] !== undefined ||
    args["trust-remove"] !== undefined ||
    args["trust-list"] === true
  );
}

async function runNonInteractive(out: Output, args: Args): Promise<void> {
  const configResult = await loadConfig();
  if (!configResult.ok) {
    out.error(configResult.error.message, configResult.error.hint);
    process.exit(1);
  }
  const config = configResult.value;

  if (args["trust-list"]) {
    if (config.security.trust.length === 0) {
      out.raw("(no trust patterns)\n");
    } else {
      for (const pattern of config.security.trust) {
        out.raw(`${pattern}\n`);
      }
    }
    return;
  }

  if (args["trust-remove"]) {
    const pattern = args["trust-remove"];
    const idx = config.security.trust.indexOf(pattern);
    if (idx === -1) {
      out.error(`No trust pattern '${pattern}' found.`);
      process.exit(1);
    }
    config.security.trust.splice(idx, 1);
    const saveResult = await saveConfig(config);
    if (!saveResult.ok) {
      out.error(saveResult.error.message);
      process.exit(1);
    }
    out.raw(`OK: removed trust pattern '${pattern}'\n`);
    return;
  }

  if (args["trust-add"]) {
    const pattern = args["trust-add"];
    if (!config.security.trust.includes(pattern)) {
      config.security.trust.push(pattern);
    }
    const saveResult = await saveConfig(config);
    if (!saveResult.ok) {
      out.error(saveResult.error.message);
      process.exit(1);
    }
    out.raw(`OK: added trust pattern '${pattern}'\n`);
    return;
  }

  if (args.scan) {
    if (!(SCAN_MODES as readonly string[]).includes(args.scan)) {
      out.error(
        `Invalid scan level: '${args.scan}'. Valid: ${SCAN_MODES.join(", ")}`,
      );
      process.exit(1);
    }
    config.security.scan = args.scan as (typeof SCAN_MODES)[number];
  }

  if (args["on-warn"]) {
    if (!(ON_WARN_MODES as readonly string[]).includes(args["on-warn"])) {
      out.error(
        `Invalid on-warn value: '${args["on-warn"]}'. Valid: ${ON_WARN_MODES.join(", ")}`,
      );
      process.exit(1);
    }
    config.security.on_warn = args["on-warn"] as (typeof ON_WARN_MODES)[number];
  }

  const saveResult = await saveConfig(config);
  if (!saveResult.ok) {
    out.error(saveResult.error.message);
    process.exit(1);
  }

  out.raw(`OK: security = ${describeSecurityMode(config.security)}\n`);
}

const SCAN_OPTIONS = [
  { value: "semantic", label: "Semantic (LLM-assisted scan)" },
  { value: "static", label: "Static (pattern-based scan)" },
  { value: "none", label: "None (skip scanning)" },
];

const ON_WARN_OPTIONS = [
  { value: "prompt", label: "Ask me (prompt)" },
  { value: "fail", label: "Always block (fail)" },
  { value: "install", label: "Install anyway (install)" },
];

async function runInteractive(out: Output): Promise<void> {
  const configResult = await loadConfig();
  if (!configResult.ok) {
    out.error(configResult.error.message, configResult.error.hint);
    process.exit(1);
  }
  const config = configResult.value;

  intro("Security Configuration");

  const scanResult = await select({
    message: "Default scan mode?",
    options: SCAN_OPTIONS,
    initialValue: config.security.scan,
  });
  if (isCancel(scanResult)) {
    cancel("Cancelled.");
    process.exit(130);
  }

  const onWarnResult = await select({
    message: "When warnings are found?",
    options: ON_WARN_OPTIONS,
    initialValue: config.security.on_warn,
  });
  if (isCancel(onWarnResult)) {
    cancel("Cancelled.");
    process.exit(130);
  }

  const trust = [...config.security.trust];

  while (true) {
    const action = await select({
      message: "Manage trust patterns?",
      options: [
        { value: "add", label: "Add a pattern" },
        { value: "remove", label: "Remove a pattern" },
        { value: "done", label: "Done" },
      ],
    });
    if (isCancel(action)) {
      cancel("Cancelled.");
      process.exit(130);
    }
    if (action === "done") break;
    if (action === "add") {
      const p = await text({
        message: "Pattern (glob; tap name or source URL):",
        validate(v) {
          if (!v) return "Required";
        },
      });
      if (isCancel(p)) {
        cancel("Cancelled.");
        process.exit(130);
      }
      const pat = p as string;
      if (!trust.includes(pat)) trust.push(pat);
    }
    if (action === "remove") {
      if (trust.length === 0) continue;
      const choice = await select({
        message: "Remove which pattern?",
        options: trust.map((p) => ({ value: p, label: p })),
      });
      if (isCancel(choice)) continue;
      const idx = trust.indexOf(choice as string);
      if (idx !== -1) trust.splice(idx, 1);
    }
  }

  let summary = describeSecurityMode({
    scan: scanResult as string,
    on_warn: onWarnResult as string,
  });
  if (trust.length > 0) {
    summary += `\n\nTrust patterns:\n  ${trust.join("\n  ")}`;
  }

  note(summary, "Security Summary");

  const saveConfirm = await confirm({
    message: "Save these settings?",
    initialValue: true,
  });
  if (isCancel(saveConfirm) || !saveConfirm) {
    cancel("Cancelled.");
    process.exit(130);
  }

  config.security = {
    ...config.security,
    scan: scanResult as (typeof SCAN_MODES)[number],
    on_warn: onWarnResult as (typeof ON_WARN_MODES)[number],
    trust,
  };

  const saveResult = await saveConfig(config);
  if (!saveResult.ok) {
    out.error(saveResult.error.message);
    process.exit(1);
  }

  outro(`Wrote ${getConfigDir()}/config.toml`);
}

export default defineCommand({
  meta: {
    name: "skilltap config security",
    description: "Configure security settings",
  },
  args: {
    scan: {
      type: "string",
      description: "Scan level: semantic, static, none",
    },
    "on-warn": {
      type: "string",
      description: "Warning behavior: prompt, fail, install",
    },
    "trust-add": {
      type: "string",
      description: "Append a glob pattern to security.trust",
    },
    "trust-remove": {
      type: "string",
      description: "Remove a glob pattern from security.trust",
    },
    "trust-list": {
      type: "boolean",
      description: "Print current trust patterns",
    },
  },
  async run({ args }) {
    const out = setupOutput({ json: false, quiet: false });

    if (isNonInteractive(args)) {
      await runNonInteractive(out, args);
      return;
    }

    if (!process.stdin.isTTY) {
      out.error(
        "'skilltap config security' requires a TTY for interactive mode. Use flags for non-interactive use.",
      );
      process.exit(1);
    }

    await runInteractive(out);
  },
});
