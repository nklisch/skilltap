import { $ } from "bun";
import { err, ok, ScanError } from "../types";
import { extractAgentResponse } from "./extract";
import type { AgentAdapter } from "./types";

export const opencodeAdapter: AgentAdapter = {
  name: "OpenCode",
  cliName: "opencode",

  async detect() {
    try {
      await $`which opencode`.quiet();
      return true;
    } catch {
      return false;
    }
  },

  async invoke(prompt) {
    try {
      const result = await $`opencode --prompt ${prompt}`.quiet();
      const raw = result.stdout.toString().trim();
      const parsed = extractAgentResponse(raw);
      if (!parsed)
        return ok({ score: 0, reason: "Could not parse agent response" });
      return ok(parsed);
    } catch (e) {
      return err(
        new ScanError(
          `OpenCode invocation failed: ${e instanceof Error ? e.message : String(e)}`,
        ),
      );
    }
  },
};
