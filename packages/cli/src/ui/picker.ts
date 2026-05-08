import { isCancel, select } from "@clack/prompts";
import type { Output } from "@skilltap/core";

export interface PickerOption<T> {
  value: T;
  label: string;
  hint?: string;
}

export async function pickOne<T>(opts: {
  message: string;
  options: PickerOption<T>[];
  emptyMessage?: string;
  out: Output;
}): Promise<T | null> {
  if (opts.options.length === 0) {
    opts.out.info(opts.emptyMessage ?? "Nothing to pick.");
    return null;
  }
  const choice = await select({
    message: opts.message,
    options: opts.options.map((o) => ({
      value: o.value,
      label: o.label,
      hint: o.hint,
    })),
  });
  if (isCancel(choice)) {
    opts.out.info("Cancelled.");
    return null;
  }
  return choice as T;
}
