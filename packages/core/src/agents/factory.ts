import { $ } from "bun";
import { type ScanError, err, ok } from "../types";
import { extractAgentResponse } from "./extract";
import type { AgentAdapter } from "./types";

type InvokeCommand = (prompt: string) => Promise<{ stdout: Buffer }>;

export function createCliAdapter(
  name: string,
  cliName: string,
  buildCommand: InvokeCommand,
): AgentAdapter {
  return {
    name,
    cliName,

    async detect() {
      try {
        await $`which ${cliName}`.quiet();
        return true;
      } catch {
        return false;
      }
    },

    async invoke(prompt) {
      try {
        const result = await buildCommand(prompt);
        const raw = result.stdout.toString().trim();
        const parsed = extractAgentResponse(raw);
        if (!parsed)
          return ok({ score: 0, reason: "Could not parse agent response" });
        return ok(parsed);
      } catch (e) {
        return err(
          new ScanError(
            `${name} invocation failed: ${e instanceof Error ? e.message : String(e)}`,
          ),
        );
      }
    },
  };
}
