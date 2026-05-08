/**
 * Low-level write helpers for fatal pre-initialization errors.
 * Used by ui/policy.ts and ui/resolve.ts which run before an Output
 * instance is available. All other output must go through the Output port.
 */
import { ansi } from "../ui/format";

export function errorLine(msg: string, hint?: string): void {
  process.stderr.write(`${ansi.red("error")}: ${msg}\n`);
  if (hint) process.stderr.write(`  ${ansi.dim("hint")}: ${hint}\n`);
}
