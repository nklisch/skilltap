import type { SemanticWarning, StaticWarning, TrustInfo } from "@skilltap/core";
import { errorLine } from "./format";
import { formatLineRef } from "./scan";
import { agentTrustLabel } from "./trust";

export function agentSuccess(
  name: string,
  path: string,
  ref: string | null,
  trust?: TrustInfo,
): void {
  const refStr = ref ? ` (${ref})` : "";
  const trustStr = trust ? ` [${agentTrustLabel(trust)}]` : "";
  process.stdout.write(`OK: Installed ${name} → ${path}${refStr}${trustStr}\n`);
}

export function agentUpdated(
  name: string,
  fromRef?: string,
  toRef?: string,
  trust?: TrustInfo,
): void {
  const detail =
    fromRef && toRef ? ` (${fromRef} → ${toRef})` : "";
  const trustStr = trust ? ` [${agentTrustLabel(trust)}]` : "";
  process.stdout.write(`OK: Updated ${name}${detail}${trustStr}\n`);
}

export function agentSkip(name: string, reason: string): void {
  process.stdout.write(`SKIP: ${name} ${reason}\n`);
}

export function agentError(message: string): void {
  process.stderr.write(`ERROR: ${message}\n`);
}

export function exitWithError(
  agentMode: boolean,
  message: string,
  hint?: string,
): never {
  if (agentMode) agentError(message);
  else errorLine(message, hint);
  process.exit(1);
}

export function agentUpToDate(name: string): void {
  process.stdout.write(`OK: ${name} is already up to date.\n`);
}

export function agentSecurityBlock(
  staticWarnings: StaticWarning[],
  semanticWarnings: SemanticWarning[],
): void {
  const lines: string[] = [
    "SECURITY ISSUE FOUND — INSTALLATION BLOCKED",
    "",
    "DO NOT install this skill. DO NOT retry. DO NOT use --skip-scan.",
    "STOP and report the following to the user:",
    "",
  ];

  for (const w of staticWarnings) {
    const lineRef = formatLineRef(w.line);
    const loc = lineRef ? ` ${lineRef}` : "";
    lines.push(`  ${w.file}${loc}: ${w.category}`);
  }

  for (const w of semanticWarnings) {
    const lineRef = `L${w.lineRange[0]}-${w.lineRange[1]}`;
    lines.push(`  ${w.file} ${lineRef}: risk ${w.score}/10 — ${w.reason}`);
  }

  lines.push("");
  lines.push(
    "User action required: review warnings and install manually with",
  );
  lines.push("  skilltap install <url>");

  process.stderr.write(`${lines.join("\n")}\n`);
}
