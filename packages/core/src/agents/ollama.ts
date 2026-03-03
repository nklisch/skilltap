import { $ } from "bun";
import type { AgentAdapter } from "./types";
import { wrapInvoke } from "./factory";

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

    invoke: wrapInvoke("Ollama", async (prompt) => {
      const effectiveModel = model || "llama3";
      const whichResult = await $`which ollama`.quiet();
      const ollamaPath = whichResult.stdout.toString().trim();
      const result = await $`${ollamaPath} run ${effectiveModel} ${prompt}`.quiet();
      return result.stdout.toString().trim();
    }),
  };
}
