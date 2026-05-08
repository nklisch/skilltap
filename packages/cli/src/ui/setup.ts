import { createOutput, type Output } from "../output";

export interface OutputArgs {
  json?: boolean;
  quiet?: boolean;
}

export function setupOutput(args: OutputArgs): Output {
  return createOutput({
    json: args.json ?? false,
    quiet: args.quiet ?? false,
  });
}
