import { ok, type Result, type UserError } from "../types";
import { createClaudeCodeScanner } from "./claude-code";
import { createCodexScanner } from "./codex";
import type { AgentPluginScanner, DiscoveredAgentPlugin } from "./types";

export function defaultScanners(): AgentPluginScanner[] {
  return [createClaudeCodeScanner(), createCodexScanner()];
}

export interface ScanAllResult {
  plugins: DiscoveredAgentPlugin[];
  /** Per-scanner errors — non-fatal; scan continues. */
  errors: { scanner: string; error: UserError }[];
}

export async function scanAllAgentPlugins(
  scanners: AgentPluginScanner[] = defaultScanners(),
): Promise<Result<ScanAllResult, UserError>> {
  const all: DiscoveredAgentPlugin[] = [];
  const errors: ScanAllResult["errors"] = [];
  for (const scanner of scanners) {
    if (!(await scanner.detect())) continue;
    const result = await scanner.scan();
    if (!result.ok) {
      errors.push({ scanner: scanner.name, error: result.error });
      continue;
    }
    all.push(...result.value);
  }
  return ok({ plugins: all, errors });
}
