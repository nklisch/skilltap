import { $ } from "bun";
import { err, ok, ScanError } from "../types";
import { extractAgentResponse } from "./extract";
import type { AgentAdapter } from "./types";

export function createCustomAdapter(binaryPath: string): AgentAdapter {
  return {
    name: `Custom (${binaryPath})`,
    cliName: binaryPath,

    async detect() {
      try {
        const file = Bun.file(binaryPath);
        return await file.exists();
      } catch {
        return false;
      }
    },

    async invoke(prompt) {
      try {
        const result = await $`echo ${prompt} | ${binaryPath}`.quiet();
        const raw = result.stdout.toString().trim();
        const parsed = extractAgentResponse(raw);
        if (!parsed)
          return ok({ score: 0, reason: "Could not parse agent response" });
        return ok(parsed);
      } catch (e) {
        return err(
          new ScanError(
            `Custom agent invocation failed: ${e instanceof Error ? e.message : String(e)}`,
          ),
        );
      }
    },
  };
}
