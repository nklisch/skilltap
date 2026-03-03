import { $ } from "bun";
import type { AgentAdapter } from "./types";
import { wrapInvoke } from "./factory";

export function createCustomAdapter(binaryPath: string): AgentAdapter {
  const name = `Custom (${binaryPath})`;
  return {
    name,
    cliName: binaryPath,

    async detect() {
      try {
        const file = Bun.file(binaryPath);
        return await file.exists();
      } catch {
        return false;
      }
    },

    invoke: wrapInvoke(name, async (prompt) => {
      const result = await $`echo ${prompt} | ${binaryPath}`.quiet();
      return result.stdout.toString().trim();
    }),
  };
}
