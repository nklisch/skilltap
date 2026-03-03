import { $ } from "bun";
import { ScanError, err, ok } from "../types";
import { extractAgentResponse } from "./extract";
import type { AgentAdapter } from "./types";

type InvokeCommand = (prompt: string) => Promise<{ stdout: Buffer }>;

/** Shared invoke wrapper: run command → parse response → Result. */
export function wrapInvoke(
  name: string,
  run: (prompt: string) => Promise<string>,
): AgentAdapter["invoke"] {
  return async (prompt) => {
    try {
      const raw = await run(prompt);
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
  };
}

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

    invoke: wrapInvoke(name, async (prompt) => {
      const result = await buildCommand(prompt);
      return result.stdout.toString().trim();
    }),
  };
}
