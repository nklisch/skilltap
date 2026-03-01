import { $ } from "bun";
import { err, ok, ScanError } from "../types";
import { extractAgentResponse } from "./extract";
import type { AgentAdapter } from "./types";

export function createOllamaAdapter(model: string): AgentAdapter {
  return {
    name: "Ollama",
    cliName: "ollama",

    async detect() {
      try {
        const whichResult = await $`which ollama`.quiet();
        const ollamaPath = whichResult.stdout.toString().trim();
        const result = await $`${ollamaPath} list`.quiet();
        // Check that at least one model is available
        const lines = result.stdout.toString().trim().split("\n");
        return lines.length > 1; // First line is header
      } catch {
        return false;
      }
    },

    async invoke(prompt) {
      try {
        const effectiveModel = model || "llama3";
        const whichResult = await $`which ollama`.quiet();
        const ollamaPath = whichResult.stdout.toString().trim();
        const result = await $`${ollamaPath} run ${effectiveModel} ${prompt}`.quiet();
        const raw = result.stdout.toString().trim();
        const parsed = extractAgentResponse(raw);
        if (!parsed)
          return ok({ score: 0, reason: "Could not parse agent response" });
        return ok(parsed);
      } catch (e) {
        return err(
          new ScanError(
            `Ollama invocation failed: ${e instanceof Error ? e.message : String(e)}`,
          ),
        );
      }
    },
  };
}
