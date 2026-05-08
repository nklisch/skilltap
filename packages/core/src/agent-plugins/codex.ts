import { ok, type Result, type UserError } from "../types";
import type { AgentPluginScanner, DiscoveredAgentPlugin } from "./types";

/**
 * Codex stub. OpenAI's Codex CLI does not ship a plugin marketplace today;
 * detect() returns false. When (if) Codex ships one, this file gets a real
 * implementation that mirrors claude-code.ts.
 */
export function createCodexScanner(): AgentPluginScanner {
  return {
    name: "codex",
    async detect(): Promise<boolean> {
      return false;
    },
    async scan(): Promise<Result<DiscoveredAgentPlugin[], UserError>> {
      return ok([]);
    },
  };
}
