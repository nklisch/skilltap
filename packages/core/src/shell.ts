import { debug } from "./debug";
import type { Result } from "./types";
import { err, ok, UserError } from "./types";

/** Extract stderr from a Bun ShellError. */
export function extractStderr(e: unknown): string {
  if (e instanceof Error && "stderr" in e) {
    const raw = (e as { stderr: unknown }).stderr;
    if (raw instanceof Uint8Array) return new TextDecoder().decode(raw).trim();
    return String(raw).trim();
  }
  return String(e);
}

/** Extract exit code from a Bun ShellError. */
export function extractExitCode(e: unknown): number | undefined {
  if (e instanceof Error && "exitCode" in e) {
    return (e as { exitCode: number }).exitCode;
  }
  return undefined;
}

/**
 * Wrap a shell command with stderr extraction and debug logging.
 * Returns Result<T, UserError> with a descriptive message on failure.
 */
export async function wrapShell<T>(
  fn: () => Promise<T>,
  msg: string,
  hint?: string,
): Promise<Result<T, UserError>> {
  try {
    return ok(await fn());
  } catch (e) {
    const stderr = extractStderr(e);
    const exitCode = extractExitCode(e);
    debug(msg, { stderr, exitCode });
    const detail = stderr || `exit code ${exitCode ?? "unknown"}`;
    return err(new UserError(`${msg}: ${detail}`, hint));
  }
}
