import { $ } from "bun";
import { err, ok, ScanError } from "../types";
import { extractAgentResponse } from "./extract";
import type { AgentAdapter } from "./types";

export const claudeAdapter: AgentAdapter = {
  name: "Claude Code",
  cliName: "claude",

  async detect() {
    try {
      await $`which claude`.quiet();
      return true;
    } catch {
      return false;
    }
  },

  async invoke(prompt) {
    try {
      const result =
        await $`claude --print -p ${prompt} --no-tools --output-format json`.quiet();
      const raw = result.stdout.toString().trim();
      const parsed = extractAgentResponse(raw);
      if (!parsed)
        return ok({ score: 0, reason: "Could not parse agent response" });
      return ok(parsed);
    } catch (e) {
      return err(
        new ScanError(
          `Claude Code invocation failed: ${e instanceof Error ? e.message : String(e)}`,
        ),
      );
    }
  },
};
