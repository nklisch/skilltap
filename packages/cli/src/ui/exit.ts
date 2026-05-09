import type { Output } from "@skilltap/core";
import type { Result } from "@skilltap/core";

interface SkilltapErrorLike {
  message: string;
  hint?: string;
}

export function exitOnError<T, E extends SkilltapErrorLike>(
  result: Result<T, E>,
  out: Output,
  options: { onError?: () => void } = {},
): asserts result is { ok: true; value: T } {
  if (!result.ok) {
    options.onError?.();
    out.error(result.error.message, result.error.hint);
    process.exit(1);
  }
}
