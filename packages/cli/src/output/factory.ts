import { type Output, type OutputOptions, pickMode } from "@skilltap/core";
import { createJsonOutput } from "./json";
import { createPlainOutput } from "./plain";
import { createTtyOutput } from "./tty";

export function createOutput(opts: OutputOptions = {}): Output {
  const mode = pickMode(opts);
  switch (mode) {
    case "tty":
      return createTtyOutput(opts);
    case "plain":
      return createPlainOutput(opts);
    case "json":
      return createJsonOutput(opts);
  }
}
